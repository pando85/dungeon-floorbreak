"""Command line of the shared pixel-art toolkit: ``python3 -m scripts.pixelart <command>``."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from . import compare, evidence, packet, review, validators
from .common import ROOT, ToolkitError, load_json, pg, rel
from .config import FORMAT, AssetConfig, default_config, load_config


def out_dir(arg: str | None, config: AssetConfig, leaf: str) -> Path:
    return Path(arg) if arg else ROOT / "generated" / "art" / "pixelart" / config.asset_id / leaf


def resolve_validate_input(path: Path, config_path: Path | None):
    """``(config, grid, manifest)`` for an asset config, a pixel-grid-v1 file or a manifest."""
    data = load_json(path)
    if config_path is not None:
        cfg = load_config(config_path)
        return cfg, None, (data if "grid_source" in data else None)
    if data.get("format") == FORMAT:
        return AssetConfig(data, path), None, None
    if data.get("format") == pg.FORMAT:
        grid = pg.load_grid(path)
        return default_config(grid, rel(path), path.name.split(".")[0]), grid, None
    if "grid_source" in data:
        grid_file = (ROOT / data["grid_source"]).resolve()
        grid = pg.load_grid(grid_file)
        cfg = default_config(grid, data["grid_source"], data.get("id", path.stem), manifest=None)
        return cfg, grid, data
    raise ToolkitError(f"{path.name}: not a pixelart-asset-v1 config, a pixel-grid-v1 grid or a manifest with grid_source")


def cmd_packet(a) -> int:
    files = packet.build_packet(
        a.asset_id, Path(a.concept), [packet.parse_crop(c) for c in a.crop], Path(a.out), colours=a.colors,
        heights=[int(h) for h in a.heights.split(",")] if a.heights else None, excerpts=a.excerpt,
        frame_size=tuple(int(v) for v in a.frame_size.split("x")), pivot=tuple(int(v) for v in a.pivot.split(",")))
    print(f"packet for {a.asset_id}:\n{packet.describe(files)}")
    return 0


def cmd_validate(a) -> int:
    cfg, grid, manifest = resolve_validate_input(Path(a.input), Path(a.config) if a.config else None)
    report = validators.run(validators.build_context(cfg, grid=grid, manifest=manifest))
    if a.out:
        validators.write_report(report, Path(a.out))
    print(json.dumps(report, indent=2, sort_keys=True) if a.json else validators.summary(report), end="" if not a.json else "\n")
    return 0 if report["passed"] else 1


def cmd_holes(a) -> int:
    found = validators.scan_holes([Path(x) for x in a.grids], a.connectivity)
    total = sum(len(v) for v in found.values())
    for grid_path, pockets in found.items():
        pinholes = sum(1 for _, pocket in pockets if len(pocket) == 1)
        frames = len({f for f, _ in pockets})
        print(f"{grid_path}: {len(pockets)} unallowed pockets ({pinholes} one-pixel) in {frames} frame(s)")
        if a.detail:
            for frame, pocket in pockets:
                print(f"    {validators.describe_hole(frame, pocket)}")
    print(f"holes: {total} unallowed pocket(s) in {len(found)} grid(s) (connectivity {a.connectivity}; allowlist {validators.HOLE_ALLOWLIST})")
    return 1 if found else 0


def cmd_evidence(a) -> int:
    cfg = load_config(a.config)
    files = evidence.build_evidence(cfg, out_dir(a.out, cfg, "evidence"))
    print(f"evidence for {cfg.asset_id}:\n{evidence.describe(files)}")
    return 0


def cmd_review_pack(a) -> int:
    cfg = load_config(a.config)
    report = validators.run(validators.build_context(cfg))
    if not report["passed"] and not a.allow_failing:
        print(validators.summary(report), end="")
        print("review-pack refused: the validators must pass before a review (use --allow-failing to write the pack anyway)",
              file=sys.stderr)
        return 1
    out = out_dir(a.out, cfg, "review_pack")
    review.build_review_pack(cfg, out, report=report)
    print(f"review pack for {cfg.asset_id} (validators: {'PASS' if report['passed'] else 'FAIL'}):")
    for name in review.IMAGE_NAMES + review.TEXT_NAMES:
        print(f"  {rel(out / name)}")
    return 0


def cmd_freeze(a) -> int:
    cfg = load_config(a.config)
    spec = cfg.rules.get("frozen_keys") or {}
    frames = [int(f) for f in a.frames.split(",")] if a.frames else list(spec.get("frames", []))
    if not frames:
        raise ToolkitError("no key frames: pass --frames or set rules.frozen_keys.frames")
    target = Path(a.out) if a.out else (ROOT / spec["fixture"] if spec.get("fixture") else None)
    if target is None:
        raise ToolkitError("no fixture path: pass --out or set rules.frozen_keys.fixture")
    if target.exists() and not a.force:
        raise ToolkitError(f"{rel(target)} exists and keys are frozen; pass --force to overwrite deliberately")
    fixture = validators.freeze(cfg, cfg.load_grid(), frames, spec.get("layers"))
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"froze {len(frames)} key frames into {rel(target)}")
    return 0


def cmd_compare(a) -> int:
    if a.preset:
        concept, sources = compare.preset_inputs(a.preset)
        title = f"{a.preset.upper()}: concept vs pixel art"
    else:
        concept, sources, title = (Path(a.concept) if a.concept else None), [Path(s) for s in a.source], ""
    if a.concept and a.preset:
        concept = Path(a.concept)
    sheet = compare.compare_sheet(concept, sources, a.scale, a.all_frames, title=title)
    out = Path(a.out) if a.out else ROOT / "generated" / "art" / "comparisons" / f"{a.preset or 'comparison'}_comparison.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {rel(out)}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="python3 -m scripts.pixelart", description=__doc__)
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("packet", help="design packet: crops, palette candidates, scale lineup, templates")
    s.add_argument("asset_id")
    s.add_argument("--concept", required=True, help="approved concept PNG")
    s.add_argument("--crop", action="append", default=[], metavar="x,y,w,h")
    s.add_argument("--out", required=True)
    s.add_argument("--colors", type=int, default=packet.MAX_COLOURS, help="palette candidates per crop (at most 24)")
    s.add_argument("--heights", help="three candidate heights, for example 40,44,48")
    s.add_argument("--excerpt", action="append", default=[], metavar="path#Heading", help="repository doc excerpt")
    s.add_argument("--frame-size", default="64x56")
    s.add_argument("--pivot", default="28,55")
    s.set_defaults(fn=cmd_packet)

    s = sub.add_parser("validate", help="run the validators on a config, a pixel-grid-v1 file or a manifest")
    s.add_argument("input")
    s.add_argument("--config", help="asset config to apply to a grid or manifest")
    s.add_argument("--out", help="directory for validator_report.json and validator_summary.txt")
    s.add_argument("--json", action="store_true", help="print the JSON report instead of the summary")
    s.set_defaults(fn=cmd_validate)

    s = sub.add_parser("holes", help="scan grids for enclosed transparent pockets (V17), allowlist applied")
    s.add_argument("grids", nargs="*", help=".grid.json files (default: every committed assets/source grid)")
    s.add_argument("--connectivity", type=int, choices=validators.HOLE_CONNECTIVITY, default=4)
    s.add_argument("--detail", action="store_true", help="list every pocket with its pixels")
    s.set_defaults(fn=cmd_holes)

    s = sub.add_parser("evidence", help="GIFs, contact sheets, strip, lit scenes and SHA256SUMS")
    s.add_argument("config")
    s.add_argument("--out")
    s.set_defaults(fn=cmd_evidence)

    s = sub.add_parser("review-pack", help="exactly three images plus the validator report")
    s.add_argument("config")
    s.add_argument("--out")
    s.add_argument("--allow-failing", action="store_true", help="write the pack although a validator fails")
    s.set_defaults(fn=cmd_review_pack)

    s = sub.add_parser("freeze", help="write the frozen-key fixture (SHA-256 per key frame)")
    s.add_argument("config")
    s.add_argument("--frames", help="comma-separated key frames (default: rules.frozen_keys.frames)")
    s.add_argument("--out")
    s.add_argument("--force", action="store_true")
    s.set_defaults(fn=cmd_freeze)

    s = sub.add_parser("compare", help="concept art next to pixel frames")
    s.add_argument("--preset", choices=sorted(compare.PRESETS))
    s.add_argument("--concept")
    s.add_argument("--source", action="append", default=[], help=".grid.json, .ase or asset config (repeatable)")
    s.add_argument("--out")
    s.add_argument("--scale", type=int, default=4)
    s.add_argument("--all-frames", action="store_true")
    s.set_defaults(fn=cmd_compare)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.fn(args)
    except ToolkitError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    except pg.GridError as exc:
        print(f"error: invalid pixel grid: {exc}", file=sys.stderr)
        return 2
