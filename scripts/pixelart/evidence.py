"""Evidence renders for one asset: per-tag GIFs at tick timing, contact sheets, tell->attack strip, lit scene.

Everything is composed from the grid source and the config's tick table; nothing is a Bevy capture, and
nothing here changes a source. Output is deterministic (no timestamps).
"""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

from . import scene
from .common import (BG, CYAN, GREY, GROUND_DARK, GROUND_LIGHT, REST_TICKS, WHITE, YELLOW, cell, gif_delays,
                     grid_images, label, rel, save_gif, sha256_file, text_width, up)
from .config import AssetConfig

TAG_COLOURS = [(90, 140, 255), (255, 214, 0), (255, 80, 80), (120, 200, 120), (255, 150, 40), (180, 120, 200),
               (0, 200, 200), (200, 120, 120)]


def key_frames(cfg: AssetConfig) -> set[int]:
    spec = cfg.rules.get("frozen_keys") or {}
    return set(spec.get("frames", []))


def tick_sequence(cfg: AssetConfig, tag: str) -> list[tuple[int, int]]:
    """``[(frame, ticks)]`` for a tag; hold frames show for REST_TICKS, and once/hold tags rest on their last frame."""
    spec = cfg.tags[tag]
    mode = spec.get("playback", "once")
    seq = [(f, t if t is not None else REST_TICKS["hold"]) for f, t in spec["frames"]]
    if mode != "loop":
        f, t = seq[-1]
        seq[-1] = (f, t + (REST_TICKS[mode] if spec["frames"][-1][1] is not None else 0))
    return seq


def sprite_on(img: Image.Image, k: int, contact_row: int | None) -> Image.Image:
    """One frame at scale k on the flat review background with the walking line under the contact row."""
    w, h = img.size
    base = Image.new("RGBA", (w, h + 4), BG + (255,))
    d = ImageDraw.Draw(base)
    line = (contact_row + 1) if contact_row is not None else h
    d.rectangle([0, line, w - 1, line], fill=GROUND_LIGHT)
    d.rectangle([0, line + 1, w - 1, h + 3], fill=GROUND_DARK)
    base.alpha_composite(img)
    return up(base, k)


def make_gifs(cfg: AssetConfig, imgs: list[Image.Image], out: Path, scales=(1, 3)) -> list[Path]:
    paths = []
    for tag in cfg.tags:
        seq = tick_sequence(cfg, tag)
        delays = gif_delays([t for _f, t in seq], cfg.tick_hz)
        for k in scales:
            path = out / f"gif_{tag}_x{k}.gif"
            save_gif([sprite_on(imgs[f], k, cfg.contact_row) for f, _t in seq], delays, path)
            paths.append(path)
    return paths


def caption(cfg: AssetConfig, frame: int, keys: set[int], full: bool) -> str:
    ticks = cfg.ticks_of(frame)
    if not full:
        return str(frame)
    return f"{frame:02d} {cfg.tag_of(frame)} {'hold' if ticks is None else str(ticks) + 't'}{' KEY' if frame in keys else ''}"


def contact_sheet(cfg: AssetConfig, imgs: list[Image.Image], k: int, max_width: int = 1800) -> Image.Image:
    """All frames at scale k, wrapped to ``max_width``; the contact row is cyan and the pivot yellow."""
    w, h = imgs[0].size
    keys = key_frames(cfg)
    full = k > 1
    gap, cap_h, pad = (6 if full else 2), (16 if full else 12), 4
    cols = max(1, min(len(imgs), (max_width - 2 * pad) // (w * k + gap)))
    rows = -(-len(imgs) // cols)
    title = f"{cfg.asset_id}: {len(imgs)} frames at x{k}; cyan = contact row {cfg.contact_row}, yellow = pivot {tuple(cfg.pivot)}"
    head = 22
    sheet = Image.new("RGB", (cols * (w * k + gap) + 2 * pad, head + rows * (h * k + cap_h + gap) + pad), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 5), title, WHITE, 11)
    for i, img in enumerate(imgs):
        r, c = divmod(i, cols)
        x, y = pad + c * (w * k + gap), head + r * (h * k + cap_h + gap)
        sheet.paste(cell(img, k, cfg.pivot, cfg.contact_row).convert("RGB"), (x, y))
        label(d, (x + 1, y + h * k + 1), caption(cfg, i, keys, full), YELLOW if i in keys else GREY, 11 if full else 9)
    return sheet


def strip_tags(cfg: AssetConfig) -> list[str]:
    configured = cfg.get("review", {}).get("strip_tags")
    if configured:
        return list(configured)
    tags: list[str] = []
    for role in ("tell", "attack"):
        tags += [t for t in cfg.role_tags(role) if t not in tags]
    return tags or list(cfg.tags)[:2]


def tell_attack_strip(cfg: AssetConfig, imgs: list[Image.Image], k: int = 4) -> Image.Image:
    """The tell -> attack frames in order at scale k, each captioned with its tick window, plus a tick bar."""
    frames: list[tuple[str, int, object]] = [(t, f, tk) for t in strip_tags(cfg) for f, tk in cfg.tags[t]["frames"]]
    w, h = imgs[0].size
    gap, pad, head, cap_h = 8, 6, 22, 34
    bar_h = 26
    sheet = Image.new("RGB", (pad * 2 + len(frames) * (w * k + gap) - gap, head + h * k + cap_h + bar_h + 28), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 5), f"{cfg.asset_id}: {' -> '.join(strip_tags(cfg))} at x{k}, tick windows at {cfg.tick_hz} Hz "
                       f"(tick 0 = first frame of the strip)", WHITE, 11)
    tick = 0
    spans = []
    for i, (tag, f, tk) in enumerate(frames):
        x = pad + i * (w * k + gap)
        sheet.paste(cell(imgs[f], k, cfg.pivot, cfg.contact_row).convert("RGB"), (x, head))
        window = "state-driven hold" if tk is None else f"ticks {tick}-{tick + tk - 1}"
        label(d, (x + 1, head + h * k + 2), f"{f:02d} {tag}", WHITE, 11)
        label(d, (x + 1, head + h * k + 17), window, YELLOW, 11)
        spans.append((tag, tk if tk is not None else 0, f))
        tick += tk or 0
    y = head + h * k + cap_h + 6
    total = max(tick, 1)
    width = sheet.width - 2 * pad
    x = pad
    names = list(dict.fromkeys(t for t, _f, _tk in frames))
    for tag, tk, f in spans:
        seg = max(int(width * tk / total), 1) if tk else 0
        if seg:
            d.rectangle([x, y, x + seg - 1, y + bar_h - 1], fill=TAG_COLOURS[names.index(tag) % len(TAG_COLOURS)])
            if seg > text_width(f"{tk}", 11) + 4:
                label(d, (x + 3, y + 6), f"{tk}", (10, 10, 20), 11)
            x += seg
    label(d, (pad, y + bar_h + 4), f"total {tick} ticks = {tick * 1000 / cfg.tick_hz:.0f} ms; " +
          ", ".join(f"{n}" for n in names), GREY, 11)
    return sheet


def write_sums(out: Path, files: list[Path]) -> Path:
    path = out / "SHA256SUMS"
    lines = sorted(f"{sha256_file(f)}  {f.relative_to(out).as_posix()}" for f in files if f.name != "SHA256SUMS")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def build_evidence(cfg: AssetConfig, out: Path, grid: dict | None = None) -> list[Path]:
    """Write the full evidence set into ``out`` and return every file (SHA256SUMS last)."""
    out.mkdir(parents=True, exist_ok=True)
    grid = grid if grid is not None else cfg.load_grid()
    imgs = grid_images(grid)
    written = make_gifs(cfg, imgs, out)
    for k in (1, 4):
        path = out / f"contact_sheet_x{k}.png"
        contact_sheet(cfg, imgs, k).save(path)
        written.append(path)
    path = out / "tell_attack_strip.png"
    tell_attack_strip(cfg, imgs).save(path)
    written.append(path)
    floor = scene.Floor()
    for name, k in (("scene_640x360.png", 1), ("scene_1920x1080.png", 3)):
        path = out / name
        scene.render_scene(cfg, imgs, k, floor).save(path)
        written.append(path)
    written.append(write_sums(out, written))
    return written


def describe(files: list[Path]) -> str:
    return "\n".join(f"  {rel(f)}" for f in files)
