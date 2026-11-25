"""Parametrized validators for one asset (rules V01..V16), driven by an :class:`AssetConfig`.

Every rule is a pure function of a :class:`Context` (config, grid, optional manifest, optional frozen-key
fixture), so it can be unit-tested on tiny synthetic grids, negative cases included. ``run`` returns a
machine-readable report; ``summary`` renders it as short text.

Rules, and the config key that parametrizes each (rules without their key are skipped, except V01 to V03
and V15, which always run):

====  ======================  ===================================================================
V01   structure               ``frame_size``, ``pivot``, tag table length, manifest frame size/pivot
V02   tags                    tag names, order and frame ranges equal the grid's tags
V03   tick table              grid ``duration_ms`` equals the tick table at ``tick_hz``; manifest rows
V04   palette                 ``rules.max_colors`` (declared and used)
V05   isolated pixels         ``rules.max_isolated_per_frame``
V06   orphan pixels           ``rules.max_orphans_per_frame``
V07   grounded contact rows   tags with ``contact: grounded`` end on ``contact_row``
V08   airborne frames         tags with ``contact: airborne`` have no pixel on ``contact_row``
V09   frozen keys             ``rules.frozen_keys`` (SHA-256 per key frame from a fixtures JSON)
V10   layers disjoint         ``rules.layers_disjoint`` (lists of layer names)
V11   canvas margin           ``rules.canvas_margin`` ``[left, top, right, bottom]``
V12   unique frames           ``rules.unique_frames``
V13   bounding boxes          ``rules.bbox`` (per-frame top/bottom/left/right/width/height limits) and the optional
                              ``rules.disc_top`` (a round body whose crown must stay on a fixed ellipse, lumps free)
V14   key counts              ``rules.key_counts`` (pixels of a palette key per frame)
V15   non-empty frames        every frame has at least one opaque pixel
V16   foot drift              ``rules.foot_drift`` (planted feet of loop tags, seam included)
V17   enclosed holes          ``rules.holes`` (transparent pixels a flood fill from the frame border cannot reach)
V18   overlay layering        the config's top-level ``overlay`` section (CG-L1..CG-L9 of the CARL-GAUNTLET-01 spec)
====  ======================  ===================================================================

V17 flags pinholes and slits: transparent pixels ringed by opaque ones, which no fill from the frame border
reaches. ``rules.holes`` is ``true`` or ``{"connectivity": 4 | 8}``. With 4 (the default) the fill moves along rows
and columns only, so a pocket whose sole exit is a diagonal still counts as enclosed (that is the visible speck of a
sprite at 1x). With 8 only fully sealed pockets count. Intended holes (the gap between an arm and the body, a hem gap
between the legs) go on the allowlist ``assets/source/pixel_hole_allowlist.json`` (``pixelart-hole-allowlist-v1``),
keyed by the grid path, with a reason each. ``python3 -m scripts.pixelart holes`` scans every committed grid.
"""
from __future__ import annotations

import math
import hashlib
import json
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path

from .common import ToolkitError, load_json, pg, rel, repo_path
from .config import AssetConfig

MAX_LISTED = 12


@dataclass
class Context:
    cfg: AssetConfig
    grid: dict
    manifest: dict | None = None
    fixture: dict | None = None
    flats: list = field(default_factory=list)       # per frame: rows of palette keys
    boxes: list = field(default_factory=list)       # per frame: (x0, y0, x1, y1) inclusive, or None

    def __post_init__(self) -> None:
        self.flats = [pg.composite_keys(self.grid, i) for i in range(len(self.grid["frames"]))]
        self.boxes = [bbox_of(f) for f in self.flats]

    def frames_of(self, rule: dict) -> list[int]:
        frames = list(rule.get("frames", []))
        for tag in rule.get("tags", []):
            if tag not in self.cfg.tags:
                raise ToolkitError(f"rule {rule.get('id', '?')}: unknown tag {tag!r}")
            frames += [f for f, _ in self.cfg.tags[tag]["frames"]]
        return sorted(set(frames))


def bbox_of(flat: list[list[str]]):
    pts = [(x, y) for y, row in enumerate(flat) for x, c in enumerate(row) if c != "."]
    if not pts:
        return None
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def frame_digest(cels: dict, layers: list[str]) -> str:
    """SHA-256 of the canonical JSON of the named layers' rows (a missing layer hashes as null)."""
    payload = {layer: cels.get(layer) for layer in sorted(layers)}
    return hashlib.sha256(json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def dig(data: dict, dotted: str):
    cur = data
    for part in dotted.split("."):
        if not isinstance(cur, dict) or part not in cur:
            raise ToolkitError(f"manifest has no {dotted!r}")
        cur = cur[part]
    return cur


def contact_spans(row: list[str]) -> list[tuple[int, int]]:
    """Runs of opaque pixels on one row, as inclusive (left, right) x pairs."""
    spans, x = [], 0
    while x < len(row):
        if row[x] == ".":
            x += 1
            continue
        start = x
        while x < len(row) and row[x] != ".":
            x += 1
        spans.append((start, x - 1))
    return spans


def foot_drift(row_a: list[str], row_b: list[str]) -> int:
    """Largest horizontal movement of a planted foot between two frames, judged on the contact row.

    Each foot is a run on the contact row; it is matched to the nearest run (by centre) of the other frame in both
    directions, and its drift is the smallest of the left-edge, right-edge and centre shifts, so a heel or toe lift
    that moves one edge is not counted as sliding. Feet that touch down or lift off (no run to match) count 0.
    Matching is one to one, nearest pairs first, so a foot that lifts off is never matched to the other foot when
    the other frame has fewer runs (a lifted swing foot on a passing frame).
    """
    a, b = contact_spans(row_a), contact_spans(row_b)
    pairs = sorted((abs((l1 + r1) - (l0 + r0)), i, j) for i, (l0, r0) in enumerate(a) for j, (l1, r1) in enumerate(b))
    used_a, used_b, worst = set(), set(), 0
    for _d, i, j in pairs:
        if i in used_a or j in used_b:
            continue
        used_a.add(i)
        used_b.add(j)
        (l0, r0), (l1, r1) = a[i], b[j]
        worst = max(worst, min(abs(l1 - l0), abs(r1 - r0), abs((l1 + r1) - (l0 + r0)) // 2))
    return worst


def v16_foot_drift(c: Context) -> list[str]:
    """Planted feet must not slide more than ``max_px`` across any step of a looping tag, the loop seam included."""
    spec = c.cfg.rules["foot_drift"]
    limit = spec["max_px"] if isinstance(spec, dict) else int(spec)
    row = c.cfg.contact_row if c.cfg.contact_row is not None else c.cfg.pivot[1]
    names = spec.get("tags") if isinstance(spec, dict) and spec.get("tags") else [
        n for n, t in c.cfg.tags.items() if t.get("playback") == "loop" and t.get("contact") == "grounded"]
    out = []
    for name in names:
        if name not in c.cfg.tags:
            raise ToolkitError(f"foot_drift: unknown tag {name!r}")
        seq = [f for f, _ in c.cfg.tags[name]["frames"]]
        for i, f in enumerate(seq):
            g = seq[(i + 1) % len(seq)]
            if len(seq) < 2 or f >= len(c.flats) or g >= len(c.flats):
                continue
            d = foot_drift(c.flats[f][row], c.flats[g][row])
            if d > limit:
                seam = " (loop seam)" if i == len(seq) - 1 else ""
                out.append(f"{name}: frame {f} -> {g}{seam}: a planted foot moves {d} px (max {limit})")
    return out


# ------------------------------------------------------------------ rules: each returns a list of failures

def v01_structure(c: Context) -> list[str]:
    out = []
    if list(c.grid["canvas"]) != c.cfg.frame_size:
        out.append(f"grid canvas {c.grid['canvas']} != frame_size {c.cfg.frame_size}")
    if list(c.grid["pivot"]) != c.cfg.pivot:
        out.append(f"grid pivot {c.grid['pivot']} != pivot {c.cfg.pivot}")
    if len(c.grid["frames"]) != c.cfg.frame_count:
        out.append(f"grid has {len(c.grid['frames'])} frames, the tag table covers {c.cfg.frame_count}")
    if c.cfg.contact_row is not None and c.cfg.contact_row != c.cfg.pivot[1]:
        out.append(f"contact_row {c.cfg.contact_row} != pivot y {c.cfg.pivot[1]}")
    if c.manifest is not None:
        if list(c.manifest.get("frame_size", [])) != c.cfg.frame_size:
            out.append(f"manifest frame_size {c.manifest.get('frame_size')} != {c.cfg.frame_size}")
        if list(c.manifest.get("pivot", [])) != c.cfg.pivot:
            out.append(f"manifest pivot {c.manifest.get('pivot')} != {c.cfg.pivot}")
    return out


def v02_tags(c: Context) -> list[str]:
    out = []
    grid_tags = [(t["name"], t["from"], t["to"]) for t in c.grid["tags"]]
    cfg_tags = [(n, t["frames"][0][0], t["frames"][-1][0]) for n, t in c.cfg.tags.items()]
    if [n for n, *_ in grid_tags] != [n for n, *_ in cfg_tags]:
        out.append(f"tag names/order differ: grid {[n for n, *_ in grid_tags]} vs config {[n for n, *_ in cfg_tags]}")
    else:
        out += [f"tag {g[0]}: grid frames {g[1]}..{g[2]} vs config {k[1]}..{k[2]}"
                for g, k in zip(grid_tags, cfg_tags) if g != k]
    if c.manifest is not None and "animation_tags" in c.manifest \
            and list(c.manifest["animation_tags"]) != list(c.cfg.tags):
        out.append("manifest animation_tags differ from the config tags")
    return out


def v03_ticks(c: Context) -> list[str]:
    out = []
    for name, tag in c.cfg.tags.items():
        for f, ticks in tag["frames"]:
            if f < len(c.grid["frames"]):
                got, want = c.grid["frames"][f]["duration_ms"], c.cfg.expected_ms(ticks)
                if got != want:
                    out.append(f"frame {f} ({name}): duration_ms {got} != {want} ({'hold' if ticks is None else f'{ticks} ticks'})")
    rows_path = c.cfg.rules.get("manifest_tick_rows")
    if rows_path:
        if c.manifest is None:
            out.append("rules.manifest_tick_rows is set but the config has no manifest")
        else:
            rows = {r["index"]: r for r in dig(c.manifest, rows_path)}
            hz_path = c.cfg.rules.get("manifest_tick_hz")
            if hz_path and dig(c.manifest, hz_path) != c.cfg.tick_hz:
                out.append(f"manifest tick_hz {dig(c.manifest, hz_path)} != {c.cfg.tick_hz}")
            for name, tag in c.cfg.tags.items():
                for f, ticks in tag["frames"]:
                    r = rows.get(f)
                    if r is None:
                        out.append(f"manifest tick row missing for frame {f}")
                    elif (r.get("tag"), r.get("ticks")) != (name, ticks):
                        out.append(f"manifest row {f}: ({r.get('tag')}, {r.get('ticks')}) != ({name}, {ticks})")
                    elif f < len(c.grid["frames"]) and r.get("duration_ms") != c.grid["frames"][f]["duration_ms"]:
                        out.append(f"manifest row {f}: duration_ms {r.get('duration_ms')} != grid {c.grid['frames'][f]['duration_ms']}")
    return out


def v04_palette(c: Context) -> list[str]:
    limit = c.cfg.rules["max_colors"]
    used = {k for flat in c.flats for row in flat for k in row} - {"."}
    out = []
    if len(c.grid["palette"]) > limit:
        out.append(f"palette declares {len(c.grid['palette'])} colours, limit {limit}")
    if len(used) > limit:
        out.append(f"{len(used)} colours used, limit {limit}")
    return out


def _lint(c: Context) -> dict:
    return pg.lint_grid(c.grid)


def v05_isolated(c: Context) -> list[str]:
    limit = c.cfg.rules["max_isolated_per_frame"]
    rep = _lint(c)
    return [f"frame {i}: {n} isolated pixels (limit {limit}), first {rep['frames'][i]['isolated_pixels'][:3]}"
            for i, n in enumerate(rep["isolated_count_per_frame"]) if n > limit]


def v06_orphans(c: Context) -> list[str]:
    limit = c.cfg.rules["max_orphans_per_frame"]
    rep = _lint(c)
    return [f"frame {i}: {n} orphan pixels (limit {limit})"
            for i, n in enumerate(rep["orphan_count_per_frame"]) if n > limit]


def _tag_frames(c: Context, contact: str) -> list[tuple[str, int]]:
    return [(n, f) for n, t in c.cfg.tags.items() if t.get("contact", "free") == contact for f, _ in t["frames"]]


def v07_grounded(c: Context) -> list[str]:
    row = c.cfg.contact_row
    out = []
    for tag, f in _tag_frames(c, "grounded"):
        box = c.boxes[f] if f < len(c.boxes) else None
        if box is not None and box[3] != row:
            out.append(f"frame {f} ({tag}): lowest opaque row {box[3]}, contact row {row}")
    return out


def v08_airborne(c: Context) -> list[str]:
    row = c.cfg.contact_row
    out = []
    for tag, f in _tag_frames(c, "airborne"):
        if f >= len(c.flats):
            continue
        if any(k != "." for k in c.flats[f][row]):
            out.append(f"frame {f} ({tag}): has pixels on contact row {row}")
        elif c.boxes[f] is not None and c.boxes[f][3] >= row:
            out.append(f"frame {f} ({tag}): lowest row {c.boxes[f][3]} is not above the contact row")
    return out


def v09_frozen(c: Context) -> list[str]:
    spec = c.cfg.rules["frozen_keys"]
    if c.fixture is None:
        return [f"frozen-key fixture {spec.get('fixture')!r} is not loaded"]
    out = []
    keys = c.fixture.get("keys", {})
    wanted = [int(k) for k in spec.get("frames", sorted(keys, key=int))]
    if sorted(int(k) for k in keys) != sorted(wanted):
        out.append(f"fixture keys {sorted(int(k) for k in keys)} != configured key frames {sorted(wanted)}")
    for n in wanted:
        entry = keys.get(str(n))
        if entry is None:
            out.append(f"key frame {n}: not in the fixture")
            continue
        layers = list(spec.get("layers") or [k for k in entry if k != "sha256"])
        want = entry.get("sha256") or frame_digest(entry, layers)
        if n >= len(c.grid["frames"]):
            out.append(f"key frame {n}: not in the grid")
            continue
        got = frame_digest(c.grid["frames"][n]["cels"], layers)
        if got != want:
            out.append(f"key frame {n}: SHA-256 {got[:12]} != frozen {want[:12]} (layers {layers})")
    return out


def v10_layers(c: Context) -> list[str]:
    out = []
    for group in c.cfg.rules["layers_disjoint"]:
        for n, frame in enumerate(c.grid["frames"]):
            seen: dict[tuple, str] = {}
            for layer in group:
                for y, row in enumerate(frame["cels"].get(layer, [])):
                    for x, ch in enumerate(row):
                        if ch != ".":
                            if (x, y) in seen:
                                out.append(f"frame {n}: layers {seen[(x, y)]} and {layer} overlap at ({x}, {y})")
                                break
                            seen[(x, y)] = layer
                    else:
                        continue
                    break
    return out


def v11_margin(c: Context) -> list[str]:
    left, top, right, bottom = c.cfg.rules["canvas_margin"]
    w, h = c.cfg.frame_size
    out = []
    for n, b in enumerate(c.boxes):
        if b is None:
            continue
        if b[0] < left or b[1] < top or b[2] > w - 1 - right or b[3] > h - 1 - bottom:
            out.append(f"frame {n}: box {b} breaks the margin {c.cfg.rules['canvas_margin']} on a {w}x{h} canvas")
    return out


def v12_unique(c: Context) -> list[str]:
    seen: dict[str, int] = {}
    out = []
    for n, flat in enumerate(c.flats):
        key = "".join("".join(r) for r in flat)
        if key in seen:
            out.append(f"frame {n} is pixel-identical to frame {seen[key]}")
        seen.setdefault(key, n)
    return out


_BOX_VALUE = {"top": lambda b: b[1], "bottom": lambda b: b[3], "left": lambda b: b[0], "right": lambda b: b[2],
              "width": lambda b: b[2] - b[0] + 1, "height": lambda b: b[3] - b[1] + 1}


def v13_bbox(c: Context) -> list[str]:
    out = []
    for rule in c.cfg.rules["bbox"]:
        for f in c.frames_of(rule):
            box = c.boxes[f] if f < len(c.boxes) else None
            if box is None:
                out.append(f"{rule.get('id', 'bbox')}: frame {f} is empty")
                continue
            for name, fn in _BOX_VALUE.items():
                v = fn(box)
                for op, ok in (("", lambda x, lim: x == lim), ("min_", lambda x, lim: x >= lim), ("max_", lambda x, lim: x <= lim)):
                    lim = rule.get(op + name)
                    if lim is not None and not ok(v, lim):
                        out.append(f"{rule.get('id', 'bbox')}: frame {f} {name} {v} violates {op + name}={lim}")
    out += _disc_top(c)
    return out


def _disc_top(c: Context) -> list[str]:
    """``rules.disc_top``: ``{id, frames|tags, ellipse: [cx, cy, rx, ry], x_range: [x0, x1], tolerance, min_on_disc}``.
    For each column in the range, the topmost opaque row is compared with the ellipse top. Lumps (ears, hats, tufts)
    may rise above it freely, but no column may sag more than ``tolerance`` px below it (the disc itself never
    drops), and at least ``min_on_disc`` columns must sit within ``tolerance`` px of it (the disc's own crown is
    visible and has not moved)."""
    out = []
    for rule in c.cfg.rules.get("disc_top", []):
        rid = rule.get("id", "disc_top")
        cx, cy, rx, ry = rule["ellipse"]
        x0, x1 = rule["x_range"]
        tol = rule.get("tolerance", 1.0)
        for f in c.frames_of(rule):
            flat = c.flats[f]
            on, worst = 0, float("-inf")
            for x in range(x0, x1 + 1):
                top = next((y for y, row in enumerate(flat) if row[x] != "."), None)
                if top is None:
                    continue
                prof = cy - ry * math.sqrt(max(0.0, 1.0 - ((x + 0.5 - cx) / rx) ** 2))
                worst = max(worst, top - prof)
                on += abs(top - prof) <= tol
            if worst > tol:
                out.append(f"{rid}: frame {f} crown sags {worst:.1f} px below the disc ellipse (tolerance {tol})")
            if on < rule.get("min_on_disc", 1):
                out.append(f"{rid}: frame {f} only {on} columns on the disc ellipse (min {rule['min_on_disc']})")
    return out


def v14_key_counts(c: Context) -> list[str]:
    out = []
    for rule in c.cfg.rules["key_counts"]:
        for f in c.frames_of(rule):
            if f >= len(c.grid["frames"]):
                out.append(f"{rule.get('id', 'key')}: frame {f} is not in the grid")
                continue
            if rule.get("layer"):
                rows = c.grid["frames"][f]["cels"].get(rule["layer"], [])
                n = sum(row.count(rule["key"]) for row in rows)
            else:
                n = sum(row.count(rule["key"]) for row in ("".join(r) for r in c.flats[f]))
            if "min" in rule and n < rule["min"]:
                out.append(f"{rule.get('id', 'key')}: frame {f} has {n} '{rule['key']}' pixels, min {rule['min']}")
            if "max" in rule and n > rule["max"]:
                out.append(f"{rule.get('id', 'key')}: frame {f} has {n} '{rule['key']}' pixels, max {rule['max']}")
    return out


def v15_nonempty(c: Context) -> list[str]:
    if c.cfg.data.get("overlay"):
        return []       # an overlay frame may be empty where the body shows no skin: CG-L9 (V18) judges it
    return [f"frame {n} is empty" for n, b in enumerate(c.boxes) if b is None]


# ------------------------------------------------------------------ V17 enclosed holes

HOLE_ALLOWLIST = "assets/source/pixel_hole_allowlist.json"
HOLE_ALLOWLIST_FORMAT = "pixelart-hole-allowlist-v1"
HOLE_CONNECTIVITY = (4, 8)


def find_holes(flat: list[list[str]], connectivity: int = 4, transparent: frozenset | set = frozenset(".")) -> list[list[tuple[int, int]]]:
    """Enclosed transparent pockets of one flattened frame, each a sorted list of ``(x, y)``.

    A flood fill from every transparent pixel on the frame border marks the open background; every transparent
    pixel it does not reach belongs to a pocket (connected components of the rest, same connectivity).
    """
    if connectivity not in HOLE_CONNECTIVITY:
        raise ToolkitError(f"holes connectivity must be one of {HOLE_CONNECTIVITY}, got {connectivity!r}")
    h = len(flat)
    w = len(flat[0]) if h else 0
    steps = [(1, 0), (-1, 0), (0, 1), (0, -1)] + ([(1, 1), (1, -1), (-1, 1), (-1, -1)] if connectivity == 8 else [])
    seen = [[False] * w for _ in range(h)]

    def fill(starts: list[tuple[int, int]]) -> list[tuple[int, int]]:
        queue = deque(starts)
        for x, y in starts:
            seen[y][x] = True
        cells = []
        while queue:
            x, y = queue.popleft()
            cells.append((x, y))
            for dx, dy in steps:
                a, b = x + dx, y + dy
                if 0 <= a < w and 0 <= b < h and not seen[b][a] and flat[b][a] in transparent:
                    seen[b][a] = True
                    queue.append((a, b))
        return cells

    fill([(x, y) for y in range(h) for x in range(w) if (x in (0, w - 1) or y in (0, h - 1)) and flat[y][x] in transparent])
    return [sorted(fill([(x, y)])) for y in range(h) for x in range(w) if flat[y][x] in transparent and not seen[y][x]]


def transparent_keys(grid: dict) -> set[str]:
    """``.`` plus every palette key whose alpha is 0."""
    return {pg.TRANSPARENT} | {p["key"] for p in grid["palette"] if pg.parse_rgba(p["rgba"])[3] == 0}


def load_hole_allowlist() -> dict:
    """``{grid path: [entry, ...]}`` from the shared allowlist file (empty when the file is absent)."""
    path = repo_path(HOLE_ALLOWLIST)
    if not path.is_file():
        return {}
    return parse_hole_allowlist(load_json(path))


def parse_hole_allowlist(data: dict) -> dict:
    """Check the allowlist document (format, a reason per entry, pixels or waive) and return ``{grid path: entries}``."""
    if data.get("format") != HOLE_ALLOWLIST_FORMAT:
        raise ToolkitError(f"{HOLE_ALLOWLIST}: format must be {HOLE_ALLOWLIST_FORMAT!r}")
    out: dict = {}
    for grid_path, entries in data.get("grids", {}).items():
        for entry in entries:
            if not str(entry.get("reason", "")).strip():
                raise ToolkitError(f"{HOLE_ALLOWLIST}: every entry of {grid_path} needs a reason")
            if not entry.get("waive") and not entry.get("pixels"):
                raise ToolkitError(f"{HOLE_ALLOWLIST}: an entry of {grid_path} needs pixels or waive")
        out[grid_path] = entries
    return out


def _entry_covers(entry: dict, frame: int, pocket: list[tuple[int, int]]) -> bool:
    frames = entry.get("frames", "all")
    if frames != "all" and frame not in frames:
        return False
    if entry.get("waive"):
        return True
    return set(pocket) <= {tuple(p) for p in entry["pixels"]}


def unallowed_holes(grid: dict, entries: list[dict], connectivity: int = 4) -> list[tuple[int, list[tuple[int, int]]]]:
    """``(frame, pocket)`` for every enclosed pocket that no allowlist entry covers entirely."""
    transparent = transparent_keys(grid)
    out = []
    for i in range(len(grid["frames"])):
        for pocket in find_holes(pg.composite_keys(grid, i), connectivity, transparent):
            if not any(_entry_covers(e, i, pocket) for e in entries):
                out.append((i, pocket))
    return out


def describe_hole(frame: int, pocket: list[tuple[int, int]]) -> str:
    cells = ", ".join(f"({x},{y})" for x, y in pocket[:6]) + (f" ... {len(pocket)} px" if len(pocket) > 6 else "")
    return f"frame {frame}: enclosed {'pinhole' if len(pocket) == 1 else 'pocket'} {cells}"


def scan_holes(paths: list[Path] | None = None, connectivity: int = 4) -> dict:
    """Enclosed pockets of every committed ``assets/source/**/*.grid.json`` (or of the given grids), allowlist applied.

    Returns ``{grid path: [(frame, pocket), ...]}`` listing only the grids with at least one unallowed pocket.
    """
    allow = load_hole_allowlist()
    grids = sorted(paths if paths else repo_path("assets/source").rglob("*.grid.json"))
    out: dict = {}
    for path in grids:
        key = rel(Path(path))
        found = unallowed_holes(pg.load_grid(Path(path)), allow.get(key, []), connectivity)
        if found:
            out[key] = found
    return out


def v17_holes(c: Context) -> list[str]:
    spec = c.cfg.rules["holes"]
    spec = spec if isinstance(spec, dict) else {}
    entries = list(spec.get("allow", [])) + list(load_hole_allowlist().get(c.cfg.data.get("grid", ""), []))
    return [describe_hole(i, pocket) for i, pocket in unallowed_holes(c.grid, entries, int(spec.get("connectivity", 4)))]


# ------------------------------------------------------------------ V18 overlay layering (CG-L1..CG-L9)

OVERLAY_DEFAULTS = {
    "layer": "gauntlet", "near": "near_arm", "watch": "watch", "pin": "pin", "prop": "prop",
    "head": "head", "hair": "hair", "skin_keys": "LMSD", "allowed_keys": "KJjTUVwH",
    "glint_key": "w", "max_glints": 3, "max_outside": 12,
}
OVERLAY_RULES = ("CG-L1", "CG-L2", "CG-L3", "CG-L4", "CG-L5", "CG-L6", "CG-L7", "CG-L8", "CG-L9")


def overlay_params(spec: dict) -> dict:
    """The overlay section of a config with the defaults filled in (``body_grid`` is required)."""
    params = {**OVERLAY_DEFAULTS, **spec}
    if not params.get("body_grid"):
        raise ToolkitError("overlay section needs 'body_grid' (the body sheet the overlay sits on)")
    return params


def cel_points(rows: list[str] | None) -> dict:
    """``{(x, y): key}`` of the opaque pixels of one cel (None or missing is empty)."""
    return {(x, y): ch for y, row in enumerate(rows or []) for x, ch in enumerate(row) if ch != "."}


def dilate1(points, w: int, h: int) -> set:
    """The point set grown by one pixel in the 8-neighbourhood, clipped to the canvas."""
    out = set(points)
    for x, y in points:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if 0 <= x + dx < w and 0 <= y + dy < h:
                    out.add((x + dx, y + dy))
    return out


def overlay_frame_findings(body_cels: dict, over_rows: list[str] | None, canvas, params: dict,
                           exceptions=()) -> dict:
    """CG-L1..CG-L7 and CG-L9 for one frame: ``{rule: [message, ...]}`` (empty list = pass).

    ``body_cels`` are the body frame's cels, ``over_rows`` the overlay cel, ``exceptions`` the skin pixels
    of the near arm that the overlay may leave uncovered (the frozen CG-L6 list of the manifest).
    """
    w, h = canvas
    over = cel_points(over_rows)
    p = params
    near = cel_points(body_cels.get(p["near"]))
    watch, pin = (set(cel_points(body_cels.get(p[k]))) for k in ("watch", "pin"))
    prop = set(cel_points(body_cels.get(p["prop"]))) - set(near)     # the stick pixels that show (the near arm is drawn over the prop)
    head_hair = set(cel_points(body_cels.get(p["head"]))) | set(cel_points(body_cels.get(p["hair"])))
    out: dict = {rule: [] for rule in OVERLAY_RULES if rule != "CG-L8"}

    def listed(points) -> str:
        pts = sorted(points)
        return str(pts[:4]) + (f" (+{len(pts) - 4})" if len(pts) > 4 else "")

    grown = dilate1(set(near), w, h)
    outside_grow = set(over) - grown
    if outside_grow:
        out["CG-L1"].append(f"{len(outside_grow)} px outside the near arm grown by 1: {listed(outside_grow)}")
    on_forbidden = set(over) & (watch | pin | prop)
    if on_forbidden:
        out["CG-L2"].append(f"{len(on_forbidden)} px cover a watch, pin or visible prop pixel: {listed(on_forbidden)}")
    off_arm = set(over) - set(near)
    on_head = off_arm & head_hair
    if on_head:
        out["CG-L3"].append(f"{len(on_head)} px outside the near arm cover head or hair: {listed(on_head)}")
    if len(off_arm) > p["max_outside"]:
        out["CG-L4"].append(f"{len(off_arm)} px outside the near arm (limit {p['max_outside']})")
    bad_keys = sorted({k for k in over.values() if k not in p["allowed_keys"]})
    if bad_keys:
        out["CG-L5"].append(f"keys outside {p['allowed_keys']}: {bad_keys}")
    glints = sum(1 for k in over.values() if k == p["glint_key"])
    if glints > p["max_glints"]:
        out["CG-L5"].append(f"{glints} '{p['glint_key']}' glint pixels (limit {p['max_glints']})")
    exposed = {q for q, k in near.items() if k in p["skin_keys"]} - watch - pin
    uncovered = exposed - set(over) - {tuple(e) for e in exceptions}
    if uncovered:
        out["CG-L6"].append(f"{len(uncovered)} near-arm skin px not covered: {listed(uncovered)}")
    edge = {q for q in over if q[0] in (0, w - 1)}
    if edge:
        out["CG-L7"].append(f"overlay pixels on the edge columns: {listed(edge)}")
    if not over and exposed - {tuple(e) for e in exceptions}:
        out["CG-L9"].append(f"empty overlay but the near arm shows {len(exposed)} skin px")
    return out


def overlay_strays(body_grid: dict, over_rows: list[str] | None, frame: int, layer: str) -> dict:
    """Orphan and isolated pixels of the body frame alone and of body + overlay: ``{"body": ..., "both": ...}``."""
    accents = {e["key"] for e in body_grid["palette"] if e.get("accent")}
    flat = pg.composite_keys(body_grid, frame)
    base_o, base_i = pg._stray_pixels(flat, accents)
    both = [list(r) for r in flat]
    for (x, y), ch in cel_points(over_rows).items():
        both[y][x] = ch
    both_o, both_i = pg._stray_pixels(both, accents)
    as_set = lambda pts: {tuple(q) for q in pts}  # noqa: E731
    return {"body": as_set(base_o) | as_set(base_i), "both": as_set(both_o) | as_set(both_i)}


def overlay_l8(body_grid: dict, over_rows: list[str] | None, frame: int, layer: str) -> tuple[list[str], list[tuple]]:
    """CG-L8: ``(failures, inherited)``: strays that body + overlay adds over the body alone, and the ones the body had."""
    s = overlay_strays(body_grid, over_rows, frame, layer)
    new = s["both"] - s["body"]
    failures = [f"{len(new)} new orphan or isolated px over the body alone: {sorted(new)[:4]}"] if new else []
    return failures, sorted(s["body"])


def load_overlay_body(cfg: AssetConfig) -> dict:
    return pg.load_grid(repo_path(overlay_params(cfg.data["overlay"])["body_grid"]))


def overlay_exceptions(c: "Context") -> dict:
    """The frozen CG-L6 exception list ``{frame: [[x, y], ...]}`` from the manifest's overlay block."""
    block = (c.manifest or {}).get("overlay", {})
    return {int(k): v for k, v in block.get("skin_exceptions", {}).items()}


def v18_overlay(c: Context) -> list[str]:
    params = overlay_params(c.cfg.data["overlay"])
    body = load_overlay_body(c.cfg)
    out: list[str] = []
    if len(body["frames"]) != len(c.grid["frames"]):
        return [f"overlay has {len(c.grid['frames'])} frames, the body sheet {len(body['frames'])}"]
    if body["canvas"] != c.grid["canvas"] or body["pivot"] != c.grid["pivot"]:
        out.append("overlay canvas or pivot differs from the body sheet")
    if [(t["name"], t["from"], t["to"]) for t in body["tags"]] != [(t["name"], t["from"], t["to"]) for t in c.grid["tags"]]:
        out.append("overlay tags differ from the body sheet")
    exceptions = overlay_exceptions(c)
    for i, frame in enumerate(c.grid["frames"]):
        rows = frame["cels"].get(params["layer"])
        found = overlay_frame_findings(body["frames"][i]["cels"], rows, c.grid["canvas"], params, exceptions.get(i, ()))
        found["CG-L8"] = overlay_l8(body, rows, i, params["layer"])[0]
        for rule in OVERLAY_RULES:
            out += [f"frame {i}: {rule} {msg}" for msg in found[rule]]
    return out


TOP_LEVEL_KEYS = ("overlay",)    # rule switches that live beside ``rules`` in the config, not inside it

# (id, name, function, config path that enables it, or None for always-on)
RULES = [
    ("V01", "structure", v01_structure, None),
    ("V02", "tags", v02_tags, None),
    ("V03", "tick table", v03_ticks, None),
    ("V04", "palette", v04_palette, "max_colors"),
    ("V05", "isolated pixels", v05_isolated, "max_isolated_per_frame"),
    ("V06", "orphan pixels", v06_orphans, "max_orphans_per_frame"),
    ("V07", "grounded contact rows", v07_grounded, None),
    ("V08", "airborne frames off the floor", v08_airborne, None),
    ("V09", "frozen keys", v09_frozen, "frozen_keys"),
    ("V10", "layers disjoint", v10_layers, "layers_disjoint"),
    ("V11", "canvas margin", v11_margin, "canvas_margin"),
    ("V12", "unique frames", v12_unique, "unique_frames"),
    ("V13", "bounding boxes", v13_bbox, "bbox"),
    ("V14", "key counts", v14_key_counts, "key_counts"),
    ("V15", "non-empty frames", v15_nonempty, None),
    ("V16", "foot drift (loop seam)", v16_foot_drift, "foot_drift"),
    ("V17", "enclosed holes", v17_holes, "holes"),
    ("V18", "overlay layering (CG-L1..CG-L9)", v18_overlay, "overlay"),
]


def load_fixture(cfg: AssetConfig) -> dict | None:
    spec = cfg.rules.get("frozen_keys")
    return load_json(repo_path(spec["fixture"])) if spec and spec.get("fixture") else None


def build_context(cfg: AssetConfig, grid: dict | None = None, manifest: dict | None = None,
                  fixture: dict | None = None) -> Context:
    grid = grid if grid is not None else cfg.load_grid()
    manifest = manifest if manifest is not None else cfg.load_manifest()
    fixture = fixture if fixture is not None else load_fixture(cfg)
    return Context(cfg, grid, manifest, fixture)


def run(ctx: Context) -> dict:
    """Run every rule; return the report (``passed`` is False when any rule fails)."""
    results = []
    for rid, name, fn, key in RULES:
        value = None if key is None else ctx.cfg.rules.get(key, ctx.cfg.data.get(key) if key in TOP_LEVEL_KEYS else None)
        enabled = key is None or not (value is None or value is False)    # 0 is a real limit
        if (rid == "V07" and not _tag_frames(ctx, "grounded")) or (rid == "V08" and not _tag_frames(ctx, "airborne")):
            enabled = False
        if not enabled and rid == "V18":
            continue        # the overlay rule is listed only for an overlay config: every other report keeps its 17 rules
        if not enabled:
            results.append({"id": rid, "name": name, "status": "skipped", "detail": "not configured", "failures": []})
            continue
        failures = fn(ctx)
        results.append({"id": rid, "name": name, "status": "fail" if failures else "pass",
                        "detail": f"{len(failures)} finding(s)" if failures else "ok",
                        "failures": failures[:MAX_LISTED] + ([f"... {len(failures) - MAX_LISTED} more"] if len(failures) > MAX_LISTED else [])})
    counts = {s: sum(1 for r in results if r["status"] == s) for s in ("pass", "fail", "skipped")}
    return {"format": "pixelart-report-v1", "asset_id": ctx.cfg.asset_id, "passed": counts["fail"] == 0,
            "frames": len(ctx.grid["frames"]), "counts": counts, "rules": results}


def summary(report: dict) -> str:
    lines = [f"pixelart validate: {report['asset_id']} ({report['frames']} frames)"]
    for r in report["rules"]:
        lines.append(f"  {r['status'].upper():7} {r['id']} {r['name']}" + ("" if r["status"] != "skipped" else " (not configured)"))
        lines += [f"            - {f}" for f in r["failures"]]
    c = report["counts"]
    lines.append(f"verdict: {'PASS' if report['passed'] else 'FAIL'} ({c['pass']} pass, {c['fail']} fail, {c['skipped']} skipped)")
    return "\n".join(lines) + "\n"


def write_report(report: dict, out_dir: Path) -> tuple[Path, Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    j, t = out_dir / "validator_report.json", out_dir / "validator_summary.txt"
    j.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    t.write_text(summary(report), encoding="utf-8")
    return j, t


def freeze(cfg: AssetConfig, grid: dict, frames: list[int], layers: list[str] | None = None) -> dict:
    """The frozen-key fixture for the chosen key frames: one SHA-256 per frame over the named layers."""
    layers = layers or list(grid["layers"])
    return {"format": "pixelart-keys-v1", "asset_id": cfg.asset_id, "layers": layers,
            "keys": {str(n): {"sha256": frame_digest(grid["frames"][n]["cels"], layers)} for n in sorted(frames)}}
