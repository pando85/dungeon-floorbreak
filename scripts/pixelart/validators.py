"""Parametrized validators for one asset (rules V01..V15), driven by an :class:`AssetConfig`.

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
V13   bounding boxes          ``rules.bbox`` (per-frame top/bottom/left/right/width/height limits)
V14   key counts              ``rules.key_counts`` (pixels of a palette key per frame)
V15   non-empty frames        every frame has at least one opaque pixel
====  ======================  ===================================================================
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path

from .common import ToolkitError, load_json, pg, repo_path
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
    return [f"frame {n} is empty" for n, b in enumerate(c.boxes) if b is None]


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
        value = None if key is None else ctx.cfg.rules.get(key)
        enabled = key is None or not (value is None or value is False)    # 0 is a real limit
        if (rid == "V07" and not _tag_frames(ctx, "grounded")) or (rid == "V08" and not _tag_frames(ctx, "airborne")):
            enabled = False
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
