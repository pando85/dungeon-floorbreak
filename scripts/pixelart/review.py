"""The review pack: exactly three images plus the validator report.

This folder is the only input a top-tier reviewer receives (with the spec). The three images are

1. ``01_contact_sheet_x4.png``       every frame at x4, contact row and pivot marked, tag and ticks captioned;
2. ``02_lit_scene_640x360_x3.png``   the lit Floor 1 composition (640x360 view) shown at x3 next to Carl and Donut;
3. ``03_tell_attack_strip.png``      the tell -> attack frames with their tick windows.

The validator report (``validator_report.json`` and ``validator_summary.txt``) sits beside them.
"""
from __future__ import annotations

from pathlib import Path

from . import evidence, scene, validators
from .common import grid_images
from .config import AssetConfig

IMAGE_NAMES = ("01_contact_sheet_x4.png", "02_lit_scene_640x360_x3.png", "03_tell_attack_strip.png")
TEXT_NAMES = ("validator_report.json", "validator_summary.txt")


def build_review_pack(cfg: AssetConfig, out: Path, grid: dict | None = None, report: dict | None = None) -> dict:
    """Write the pack into ``out`` (created, existing pack files replaced) and return the validator report."""
    out.mkdir(parents=True, exist_ok=True)
    for stale in IMAGE_NAMES + TEXT_NAMES:
        (out / stale).unlink(missing_ok=True)
    grid = grid if grid is not None else cfg.load_grid()
    imgs = grid_images(grid)
    if report is None:
        report = validators.run(validators.build_context(cfg, grid=grid))
    evidence.contact_sheet(cfg, imgs, 4).save(out / IMAGE_NAMES[0])
    scene.render_scene(cfg, imgs, 3).save(out / IMAGE_NAMES[1])
    evidence.tell_attack_strip(cfg, imgs).save(out / IMAGE_NAMES[2])
    validators.write_report(report, out)
    return report
