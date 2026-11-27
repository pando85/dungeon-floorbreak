#!/usr/bin/env python3
"""TROG-FX-01 authoring record: the troglodytes' death parts, cut from the approved TROG-01/02/03 idle frame 0.

Usage:

    python3 scripts/author_trogfx01.py sheets [--check]
    python3 scripts/author_trogfx01.py debug OUT_DIR
    python3 scripts/author_trogfx01.py contact OUT.png

``sheets`` (re)writes, for each of the Pygmy, the Basher and the Virtuoso, a one-frame pose grid (the idle frame 0, the
reference the parts re-tile exactly), a parts grid (the part cuts, the gibs and the blood drop), their ``.ase`` sources,
their ``pixelart-asset-v1`` configs and their manifests under ``assets/source/characters/troglodyte/death/`` and
``assets/manifests/``. ``debug`` renders each cut at x8 (review only, not committed). The cut rules are
``docs/02-art/trog-fx-01-spec.md`` section 2; every number below is a pinned measurement on the committed idle frame 0.

Construction (no concept pixels, no tracing, no new character key): each opaque pixel of the approved idle frame 0 goes
to exactly one part by the ordered region rules of ``CUTS``; a part is its pixels moved, unchanged, into a cell with its
bounding-box centre on the cell centre (``frame0_origin`` is that centre in frame 0 pixels), so the parts re-tile the
sprite to the pixel. The gore layer adds a wound rim on every cut face (dark clot, a few wet reds, one bright pixel). The
gibs and the blood drop are the GOB-02 cels recoloured to the troglodyte skin keys. Presentation-only art tooling.
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

CHAR = ROOT / "assets/source/characters/troglodyte"
OUT = CHAR / "death"
MANIFESTS = ROOT / "assets/manifests"
GOB_PARTS = ROOT / "assets/source/characters/goblin/death/goblin_a_death_parts.grid.json"
SPEC = "docs/02-art/trog-fx-01-spec.md"
LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
           "no game/asset-pack distribution rights")
PALETTE_ID = "trog-death-floor01-v0"
GORE_KEYS = [("R", "red_mid", "#b8322eff", False), ("B", "blood_dark", "#6e1a22ff", False),
             ("P", "blood_wet", "#e2434bff", True)]
CELL_STEP = 8
MIN_CELL = 32
PRESETS = {  # the GOB-02 preset filters of the same part kinds (spec section 4)
    "head": [1, 2, 3, 4], "torso_upper": [2, 3, 4], "torso_lower": [4], "arm_near": [3, 4], "arm_far": [4],
    "leg_near": [3, 4], "leg_far": [3, 4], "bell": [1, 2, 3, 4],
}
FLESH = {"mass": "heavy", "spin": "slow", "settle": "flop", "bounce": 0.2}
PART_FLAGS = {  # id -> (class, mass, spin, settle, bounce, ttl)
    "head": ("wound", "medium", "fast", "roll", 0.35, "keep"),
    "torso_upper": ("wound", "heavy", "slow", "flop", 0.2, "keep"),
    "torso_lower": ("wound", "heavy", "slow", "flop", 0.2, "keep"),
    "arm_near": ("wound", "light", "fast", "flop", 0.3, "long"),
    "arm_far": ("wound", "light", "fast", "flop", 0.3, "long"),
    "leg_near": ("wound", "medium", "slow", "flop", 0.25, "long"),
    "leg_far": ("wound", "medium", "slow", "flop", 0.25, "long"),
    "bell": ("clean", "light", "fast", "clatter", 0.3, "keep"),
}


def inside(x: int, y: int, x0=-10**6, x1=10**6, y0=-10**6, y1=10**6) -> bool:
    return x0 <= x <= x1 and y0 <= y <= y1


# Ordered rules per species: the first rule that holds takes the pixel; ``REST`` names the catch-all. Coordinates are
# idle frame 0 pixels (x right, y down). ``key`` is the palette key of the pixel.
CUTS = {
    "pygmy": {
        "grid": CHAR / "trog_01.grid.json", "stem": "trog_pygmy", "label": "Pygmy",
        "rules": [
            ("head", lambda x, y, k: x >= 28 and y <= 36 or (x >= 22 and y <= 27)),
            ("arm_near", lambda x, y, k: x >= 36 and y >= 37),
            ("leg_near", lambda x, y, k: x >= 24 and y >= 48),
            ("torso_lower", lambda x, y, k: x >= 16 and y >= 41),
        ],
        "rest": "torso_upper",
    },
    "basher": {
        "grid": CHAR / "basher/trog_02.grid.json", "stem": "trog_basher", "label": "Basher",
        "rules": [
            ("head", lambda x, y, k: x >= 38 and y <= 45),
            ("arm_near", lambda x, y, k: x >= 63 and y >= 56),
            ("arm_far", lambda x, y, k: x >= 51 and y >= 58),
            ("leg_far", lambda x, y, k: x <= 31 and y >= 62),
            ("leg_near", lambda x, y, k: 36 <= x <= 50 and y >= 72),
            ("torso_lower", lambda x, y, k: y >= 58),
        ],
        "rest": "torso_upper",
    },
    "virtuoso": {
        "grid": CHAR / "virtuoso/trog_03.grid.json", "stem": "trog_virtuoso", "label": "Virtuoso",
        "rules": [
            ("head", lambda x, y, k: y <= 44),
            ("bell", lambda x, y, k: x <= 29 and 65 <= y <= 75),
            ("arm_near", lambda x, y, k: x >= 52 and y >= 60),
            ("leg_near", lambda x, y, k: x >= 40 and y >= 62),
            ("leg_far", lambda x, y, k: x <= 40 and y >= 80),
            ("torso_lower", lambda x, y, k: y >= 62),
        ],
        "rest": "torso_upper",
    },
}
PART_ORDER = ["head", "torso_upper", "torso_lower", "arm_near", "arm_far", "leg_near", "leg_far", "bell"]
GENERIC = ["gib_flesh_a", "gib_flesh_b", "gib_bone", "gib_rag", "drop_blood"]
# GOB-02 gib keys -> the troglodyte skin keys; the rag keys -> the species' cloth keys.
SKIN_MAP = {"a": "d", "s": "s", "m": "m", "l": "l", "K": "K", "w": "w"}
RAG_MAP = {"pygmy": {"r": "c", "t": "c", "u": "C"}, "basher": {"r": "c", "t": "c", "u": "C"},
           "virtuoso": {"r": "u", "t": "v", "u": "V"}}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def pix_of(rows: list[str]) -> dict:
    return {(x, y): ch for y, row in enumerate(rows) for x, ch in enumerate(row) if ch != "."}


def rows_of(pix: dict, size: tuple[int, int]) -> list[str]:
    w, h = size
    grid = [["."] * w for _ in range(h)]
    for (x, y), ch in pix.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel outside the {w}x{h} canvas: {(x, y)}")
        grid[y][x] = ch
    return ["".join(r) for r in grid]


def assign(species: str) -> dict:
    """Every opaque pixel of the idle frame 0 -> its part id."""
    spec = CUTS[species]
    body = load(spec["grid"])["frames"][0]["cels"]["body"]
    out = {}
    for (x, y), key in pix_of(body).items():
        for part, rule in spec["rules"]:
            if rule(x, y, key):
                out[(x, y)] = part
                break
        else:
            out[(x, y)] = spec["rest"]
    return heal(out)


NEIGHBOURS8 = [(dx, dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx, dy) != (0, 0)]


def heal(owner: dict) -> dict:
    """A scrap of 1 or 2 pixels cut off from its part joins the neighbouring part it touches most (V05)."""
    changed = True
    while changed:
        changed = False
        seen: set = set()
        for start in sorted(owner):
            if start in seen:
                continue
            part = owner[start]
            comp, stack = [], [start]
            seen.add(start)
            while stack:
                x, y = stack.pop()
                comp.append((x, y))
                for dx, dy in NEIGHBOURS8:
                    q = (x + dx, y + dy)
                    if q in owner and q not in seen and owner[q] == part:
                        seen.add(q)
                        stack.append(q)
            if len(comp) > 2:
                continue
            votes: dict = {}
            for x, y in comp:
                for dx, dy in NEIGHBOURS8:
                    q = (x + dx, y + dy)
                    if q in owner and owner[q] != part:
                        votes[owner[q]] = votes.get(owner[q], 0) + 1
            if votes:
                best = max(sorted(votes), key=lambda k: votes[k])
                for c in comp:
                    owner[c] = best
                changed = True
    return owner


def parts_of(species: str) -> dict[str, dict]:
    """Part id -> {pix: {(x, y): key} in frame 0 pixels, bbox, origin}."""
    spec = CUTS[species]
    body = pix_of(load(spec["grid"])["frames"][0]["cels"]["body"])
    owner = assign(species)
    out: dict[str, dict] = {}
    for pid in PART_ORDER:
        pix = {p: body[p] for p, o in owner.items() if o == pid}
        if not pix:
            continue
        xs = [x for x, _ in pix]
        ys = [y for _, y in pix]
        box = (min(xs), min(ys), max(xs), max(ys))
        out[pid] = {"pix": pix, "bbox": box, "origin": ((box[0] + box[2]) // 2, (box[1] + box[3]) // 2)}
    return out


def cell_of(species: str) -> int:
    big = max(max(p["bbox"][2] - p["bbox"][0] + 1, p["bbox"][3] - p["bbox"][1] + 1) for p in parts_of(species).values())
    need = big + 4
    cell = MIN_CELL
    while cell < need:
        cell += CELL_STEP
    return cell


def wound_rim(species: str, pid: str, owner: dict) -> dict:
    """The gore layer of a cut part: a dark clot on every pixel facing another part (outline pixels excepted)."""
    if pid == "bell":
        return {}                                   # clean brass: a prop, not flesh
    part = parts_of(species)[pid]
    rim = {}
    n = 0
    for (x, y), key in sorted(part["pix"].items()):
        if key == "K":
            continue
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            other = owner.get((x + dx, y + dy))
            if other is not None and other != pid:
                n += 1
                rim[(x, y)] = "R" if n % 3 == 0 else "B"
                break
    if rim:
        wet = sorted(rim)[len(rim) // 2]
        rim[wet] = "P"
    return rim


def palette_for(species: str, keys: set[str]) -> list[dict]:
    base = {p["key"]: p for p in load(CUTS[species]["grid"])["palette"]}
    out = [dict(base[k]) for k in sorted(keys & set(base))]
    out += [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if a else {}))
            for k, n, c, a in GORE_KEYS if k in keys]
    return out


def recolour(rows: list[str], mapping: dict) -> dict:
    out = {}
    for p, ch in pix_of(rows).items():
        out[p] = mapping.get(ch, ch)
    return out


def generic_cels(species: str, gid: str, cell: int) -> dict:
    gob = load(GOB_PARTS)
    index = {t["name"]: t["from"] for t in gob["tags"]}[gid]
    cels = gob["frames"][index]["cels"]
    shift = (cell - 32) // 2
    mapping = dict(SKIN_MAP)
    mapping.update(RAG_MAP[species])

    def moved(rows: list[str], recolour_body: bool) -> list[str]:
        pix = {}
        for (x, y), ch in pix_of(rows).items():
            pix[(x + shift, y + shift)] = mapping.get(ch, ch) if recolour_body else ch
        return rows_of(pix, (cell, cell))

    return {"body": moved(cels["body"], True), "prop": rows_of({}, (cell, cell)), "gore": moved(cels["gore"], False)}


def parts_grid(species: str) -> dict:
    spec = CUTS[species]
    cell = cell_of(species)
    parts = parts_of(species)
    owner = assign(species)
    frames, tags, keys = [], [], set()
    ids = list(parts) + GENERIC
    for i, pid in enumerate(ids):
        if pid in parts:
            p = parts[pid]
            ox, oy = p["origin"]
            c = cell // 2
            body = {(x - ox + c, y - oy + c): ch for (x, y), ch in p["pix"].items()}
            gore = {(x - ox + c, y - oy + c): ch for (x, y), ch in wound_rim(species, pid, owner).items()}
            cels = {"body": rows_of(body, (cell, cell)), "prop": rows_of({}, (cell, cell)),
                    "gore": rows_of(gore, (cell, cell))}
        else:
            cels = generic_cels(species, pid, cell)
        for rows in cels.values():
            keys |= {ch for row in rows for ch in row if ch != "."}
        frames.append({"duration_ms": 100, "cels": cels})
        tags.append({"name": pid, "from": i, "to": i, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [cell, cell], "pivot": [cell // 2, cell // 2],
            "palette": palette_for(species, keys), "layers": ["body", "prop", "gore"], "frames": frames, "tags": tags}


def pose_grid(species: str) -> dict:
    src = load(CUTS[species]["grid"])
    body = src["frames"][0]["cels"]["body"]
    keys = {ch for row in body for ch in row if ch != "."}
    empty = ["." * len(body[0])] * len(body)
    return {"format": "pixel-grid-v1", "canvas": src["canvas"], "pivot": src["pivot"],
            "palette": palette_for(species, keys), "layers": ["body", "prop", "gore"],
            "frames": [{"duration_ms": 100, "cels": {"body": body, "prop": empty, "gore": empty}}],
            "tags": [{"name": "idle0", "from": 0, "to": 0, "direction": "forward"}]}


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def paths(species: str) -> dict:
    stem = CUTS[species]["stem"]
    return {"pose": (OUT / f"{stem}_death_pose.grid.json", OUT / f"{stem}_death_pose.ase",
                     OUT / f"{stem}_death_pose.pixelart.json", MANIFESTS / f"{stem}_death_pose.json", f"{stem}_death_pose"),
            "parts": (OUT / f"{stem}_death_parts.grid.json", OUT / f"{stem}_death_parts.ase",
                      OUT / f"{stem}_death_parts.pixelart.json", MANIFESTS / f"{stem}_death_parts.json",
                      f"{stem}_death_parts")}


def base_manifest(species: str, kind: str, grid: dict) -> dict:
    g, ase, _cfg, _man, stem = paths(species)[kind]
    label = CUTS[species]["label"]
    return {
        "schema_version": 1, "id": f"{stem}_v1", "type": "character_animation",
        "source_path": rel(ase), "grid_source": rel(g), "output_atlas": f"generated/art/{stem}.png",
        "output_metadata": f"generated/art/{stem}.json", "palette_id": PALETTE_ID,
        "frame_size": grid["canvas"], "pivot": grid["pivot"], "animation_tags": [t["name"] for t in grid["tags"]],
        "required_alpha": True,
        "owner": f"AI-authored enemy art (fan-concept layer: Floor 1 troglodyte {label}; runtime role IP-neutral)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": (f"TROG-FX-01 ({SPEC}): the {label}'s death {'reference pose' if kind == 'pose' else 'parts'} cut by "
                       "scripts/author_trogfx01.py from the committed TROG idle frame 0 (parts re-tile the sprite; gibs and "
                       "the blood drop are the GOB-02 cels recoloured); no image-model output, no tracing, no new key"),
            "generator": ("Cut rules and spec: Claude Sonnet (execution tier, reuse of approved pixels). "
                          "Independent review by Claude Opus pending"),
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
    }


def pose_manifest(species: str) -> dict:
    grid = pose_grid(species)
    m = base_manifest(species, "pose", grid)
    m["death_poses"] = {
        "spec": SPEC, "variant": "TROG", "tick_hz": 60, "hold_preview_ms": 100, "facing": "right",
        "mirror": "x' = W - x around the pivot column", "layers": grid["layers"],
        "status": "executor pass complete; independent review pending; wired into the Bevy presenter",
        "frames": [{"index": 0, "tag": "idle0", "ticks": None, "duration_ms": 100, "author": "executor", "anchors": {}}],
    }
    return m


def parts_manifest(species: str) -> dict:
    grid = parts_grid(species)
    m = base_manifest(species, "parts", grid)
    cell = grid["canvas"][0]
    parts = parts_of(species)
    out = []
    for i, t in enumerate(grid["tags"]):
        pid = t["name"]
        if pid in parts:
            cls, mass, spin, settle, bounce, ttl = PART_FLAGS[pid]
            out.append({"id": pid, "frame": i, "cell": [cell, cell], "pivot": [cell // 2, cell // 2], "anchors": {},
                        "frame0_origin": list(parts[pid]["origin"]), "class": cls,
                        "material": "brass" if pid == "bell" else "troglodyte_flesh", "presets": PRESETS[pid],
                        "gore_layer": pid != "bell", "mass": mass, "spin": spin, "settle": settle, "bounce": bounce,
                        "stain": None if pid == "bell" else "decal_splat_s", "ttl": ttl})
            if out[-1]["stain"] is None:
                del out[-1]["stain"]
            continue
        out.append(generic_meta(pid, i, cell))
    m["death_parts"] = {
        "spec": SPEC, "variant": "TROG", "cell": [cell, cell], "pivot": [cell // 2, cell // 2],
        "layers": grid["layers"], "mirror": f"x' = {cell} - x around the cell pivot", "status":
        "executor pass complete; independent review pending; wired into the Bevy presenter", "key_frames": [],
        "preset_filter": {"1": "clean parts and head", "2": "wound (gore muted), clean, blood drops",
                          "3": "wound, clean, gib, blood", "4": "all"},
        "base_spec": "docs/02-art/gob-02-death-spec.md", "always_shown_layers": ["body", "prop"], "parts": out,
    }
    return m


def generic_meta(pid: str, frame: int, cell: int) -> dict:
    gob = {p["id"]: p for p in load(MANIFESTS / "goblin_a_death_parts.json")["death_parts"]["parts"]}
    p = copy.deepcopy(gob[pid])
    p["frame"] = frame
    p["cell"] = [cell, cell]
    p["pivot"] = [cell // 2, cell // 2]
    shift = (cell - 32) // 2
    p["anchors"] = {k: [v[0] + shift, v[1] + shift] for k, v in p["anchors"].items()}
    p["material"] = "troglodyte_flesh" if p["material"] == "goblin_flesh" else p["material"]
    return p


def config(species: str, kind: str) -> dict:
    g, _ase, cfg, man, stem = paths(species)[kind]
    grid = pose_grid(species) if kind == "pose" else parts_grid(species)
    import pixel_grid_to_ase as pg
    rep = pg.lint_grid(grid)
    n = len(grid["frames"])
    label = CUTS[species]["label"]
    return {
        "format": "pixelart-asset-v1", "asset_id": stem.replace("_", "-"),
        "title": (f"TROG-FX-01 {label} death {'reference pose' if kind == 'pose' else 'parts sheet'} ({SPEC}); "
                  f"{n} frame{'s' if n != 1 else ''}"),
        "grid": rel(g), "manifest": rel(man),
        "frame_size": grid["canvas"], "pivot": grid["pivot"], "contact_row": None, "tick_hz": 60, "hold_ms": 100,
        "facing": "right",
        "rules": {"max_colors": 20, "max_isolated_per_frame": 0,
                  "max_orphans_per_frame": max(rep["orphan_count_per_frame"]),
                  "canvas_margin": [0, 0, 0, 0], "unique_frames": True, "holes": False},
        "tags": {t["name"]: {"playback": "hold", "contact": "free", "frames": [[t["from"], None]]}
                 for t in grid["tags"]},
    }


def outputs() -> list[tuple[Path, Path, dict, Path, dict, Path, dict]]:
    outs = []
    for species in CUTS:
        for kind, grid, man in (("pose", pose_grid(species), pose_manifest(species)),
                                ("parts", parts_grid(species), parts_manifest(species))):
            g, ase, cfg, mpath, _stem = paths(species)[kind]
            outs.append((g, ase, grid, mpath, man, cfg, config(species, kind)))
    return outs


def debug(out_dir: Path) -> None:
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    colours = {"head": (230, 80, 80), "torso_upper": (80, 200, 90), "torso_lower": (80, 140, 230),
               "arm_near": (240, 200, 60), "arm_far": (200, 90, 220), "leg_near": (60, 220, 220),
               "leg_far": (240, 140, 60), "bell": (255, 255, 255)}
    for species in CUTS:
        owner = assign(species)
        body = pix_of(load(CUTS[species]["grid"])["frames"][0]["cels"]["body"])
        xs = [x for x, _ in body]
        ys = [y for _, y in body]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        S = 8
        im = Image.new("RGB", ((x1 - x0 + 1) * S, (y1 - y0 + 1) * S), (30, 30, 40))
        for (x, y), part in owner.items():
            c = colours[part]
            for dy in range(S):
                for dx in range(S):
                    im.putpixel(((x - x0) * S + dx, (y - y0) * S + dy), c)
        im.save(out_dir / f"{species}_cuts.png")
        parts = parts_of(species)
        print(species, "cell", cell_of(species), {k: (v["bbox"], len(v["pix"])) for k, v in parts.items()})


def contact(out: Path, scale: int = 4) -> None:
    """The parts of the three kinds at x4 (body and gore layers, the preset 4 look), one row per kind, each part in its
    own cell with its id; the pose (idle frame 0) leads the row. Review evidence, byte-deterministic."""
    from PIL import Image, ImageDraw
    pad, label_h = 6, 12
    rows = []
    for species in CUTS:
        grid = parts_grid(species)
        pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for p in grid["palette"]}
        cell = grid["canvas"][0]
        pose = pose_grid(species)
        pw, ph = pose["canvas"]
        pal_pose = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for p in pose["palette"]}
        tiles = [(f"{CUTS[species]['label']} idle 0", pose["frames"][0]["cels"], pw, ph, pal_pose)]
        tiles += [(t["name"], grid["frames"][t["from"]]["cels"], cell, cell, pal) for t in grid["tags"]]
        rows.append(tiles)
    width = max(sum(t[2] * scale + pad for t in tiles) for tiles in rows) + pad
    height = sum(max(t[3] for t in tiles) * scale + label_h + pad for tiles in rows) + pad
    im = Image.new("RGB", (width, height), (26, 22, 34))
    draw = ImageDraw.Draw(im)
    y = pad
    for tiles in rows:
        x = pad
        h = max(t[3] for t in tiles) * scale
        for name, cels, w, hh, pal in tiles:
            draw.rectangle([x, y + label_h, x + w * scale - 1, y + label_h + hh * scale - 1], fill=(40, 34, 52))
            for layer in ("body", "prop", "gore"):
                for (px, py), ch in pix_of(cels[layer]).items():
                    draw.rectangle([x + px * scale, y + label_h + py * scale, x + px * scale + scale - 1,
                                    y + label_h + py * scale + scale - 1], fill=pal[ch])
            draw.text((x, y), name, fill=(220, 210, 190))
            x += w * scale + pad
        y += h + label_h + pad
    out.parent.mkdir(parents=True, exist_ok=True)
    im.save(out, optimize=False)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["sheets", "debug", "contact"])
    parser.add_argument("out", nargs="?")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    if args.command == "contact":
        contact(Path(args.out or "contact_sheet_x4.png"))
        return 0
    if args.command == "debug":
        debug(Path(args.out or "."))
        return 0
    outs = outputs()
    if args.check:
        same = all(g.is_file() and g.read_text(encoding="utf-8") == dump(grid)
                   and m.is_file() and m.read_text(encoding="utf-8") == json.dumps(man, indent=2) + "\n"
                   and c.is_file() and c.read_text(encoding="utf-8") == json.dumps(cfg, indent=1) + "\n"
                   for g, _a, grid, m, man, c, cfg in outs)
        print("TROG-FX-01 sheets up to date" if same else "TROG-FX-01 sheets differ from the generator")
        return 0 if same else 1
    import pixel_grid_to_ase as pg
    OUT.mkdir(parents=True, exist_ok=True)
    for g, ase, grid, m, man, c, cfg in outs:
        g.write_text(dump(grid), encoding="utf-8")
        pg.write_ase(grid, ase)
        m.write_text(json.dumps(man, indent=2) + "\n", encoding="utf-8")
        c.write_text(json.dumps(cfg, indent=1) + "\n", encoding="utf-8")
        print(f"wrote {rel(g)}, {rel(ase)}, {rel(m)}, {rel(c)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
