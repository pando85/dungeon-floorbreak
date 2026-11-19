"""The review pack: exactly three images plus the validator report.

This folder is the only input a top-tier reviewer receives (with the spec). The three images are

1. ``01_contact_sheet_x4.png``       every frame at x4, contact row and pivot marked, tag and ticks captioned;
2. ``02_lit_scene_640x360_x3.png``   the lit Floor 1 composition (640x360 view) shown at x3 next to Carl and Donut;
3. ``03_tell_attack_strip.png``      the tell -> attack frames with their tick windows (an asset without attacks sets
   ``review.strip_name`` in its config, for example ``03_jump_land_strip.png``; the strip shows ``review.strip_tags``).

The validator report (``validator_report.json`` and ``validator_summary.txt``) sits beside them.
"""
from __future__ import annotations

from pathlib import Path

from . import evidence, scene, validators
from .common import grid_images
from .config import AssetConfig

IMAGE_NAMES = ("01_contact_sheet_x4.png", "02_lit_scene_640x360_x3.png", "03_tell_attack_strip.png")
TEXT_NAMES = ("validator_report.json", "validator_summary.txt")


def image_names(cfg: AssetConfig) -> tuple[str, str, str]:
    """The three pack image names; the strip name may be overridden by ``review.strip_name``."""
    strip = cfg.get("review", {}).get("strip_name") or IMAGE_NAMES[2]
    return IMAGE_NAMES[0], IMAGE_NAMES[1], strip


def build_review_pack(cfg: AssetConfig, out: Path, grid: dict | None = None, report: dict | None = None) -> dict:
    """Write the pack into ``out`` (created, existing pack files replaced) and return the validator report."""
    out.mkdir(parents=True, exist_ok=True)
    names = image_names(cfg)
    for stale in tuple(dict.fromkeys(IMAGE_NAMES + names)) + TEXT_NAMES:
        (out / stale).unlink(missing_ok=True)
    grid = grid if grid is not None else cfg.load_grid()
    imgs = grid_images(grid)
    if report is None:
        report = validators.run(validators.build_context(cfg, grid=grid))
    evidence.contact_sheet(cfg, imgs, 4).save(out / names[0])
    scene.render_scene(cfg, imgs, 3).save(out / names[1])
    evidence.tell_attack_strip(cfg, imgs).save(out / names[2])
    validators.write_report(report, out)
    return report
