"""Shared helpers: repository paths, grid rendering, tick timing, fonts."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent.parent
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import pixel_grid_to_ase as pg  # noqa: E402  (the existing converter, lint and composite code)

BG = (24, 23, 34)
GROUND_LIGHT, GROUND_DARK = (74, 68, 100), (52, 48, 74)
CYAN, YELLOW, WHITE, GREY = (0, 220, 255), (255, 214, 0), (236, 232, 240), (150, 146, 170)
CELL_BG = (36, 34, 50)

# Approved Floor 1 references used for scale: (grid source, native idle height in px)
REFERENCES = {
    "carl": ("assets/source/characters/carl/carl_h52.grid.json", 52),
    "donut": ("assets/source/characters/donut/donut_h40.grid.json", 40),
    "goblin": ("assets/source/characters/goblin/goblin_a_h36.grid.json", 36),
}
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
DONUT_X, CARL_X = 404, 452          # walking-line x positions in the Floor 1 layout (view x 328..968)
SCENE_X = (520, 580, 640, 700, 760, 820)

TICK_HZ = 60
REST_TICKS = {"hold": 30, "once": 20, "loop": 0}


class ToolkitError(RuntimeError):
    """A user-facing failure (bad config, missing input). The CLI prints it without a traceback."""


def repo_path(raw: str) -> Path:
    """Resolve a repository-relative path and refuse anything that leaves the repository."""
    if not raw or Path(raw).is_absolute():
        raise ToolkitError(f"path must be repository-relative: {raw!r}")
    path = (ROOT / raw).resolve()
    if not path.is_relative_to(ROOT):
        raise ToolkitError(f"path escapes the repository: {raw!r}")
    return path


def load_json(path: Path) -> dict:
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ToolkitError(f"cannot read JSON {path}: {exc}") from exc


def sha256_file(path: Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def rel(path: Path) -> str:
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else path.name


# ------------------------------------------------------------------ grids and images

def palette_rgba(grid: dict) -> dict[str, tuple[int, int, int, int]]:
    return {p["key"]: pg.parse_rgba(p["rgba"]) for p in grid["palette"]}


def grid_images(grid: dict) -> list[Image.Image]:
    """Every frame of a grid as a flattened RGBA image (top layer wins), authored size."""
    pal = palette_rgba(grid)
    w, h = grid["canvas"]
    out = []
    for i in range(len(grid["frames"])):
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = img.load()
        for y, row in enumerate(pg.composite_keys(grid, i)):
            for x, key in enumerate(row):
                if key != ".":
                    px[x, y] = pal[key]
        out.append(img)
    return out


def load_grid_file(path: Path) -> dict:
    """A pixel-grid-v1 JSON or an Aseprite .ase file as a validated grid."""
    path = Path(path)
    try:
        if path.suffix in (".ase", ".aseprite"):
            return pg.normalize(pg.ase_to_grid(path.read_bytes()))
        return pg.load_grid(path)
    except (OSError, ValueError, KeyError) as exc:
        raise ToolkitError(f"cannot load grid {rel(path)}: {exc}") from exc


def flip(img: Image.Image, pivot: tuple[int, int]) -> tuple[Image.Image, tuple[int, int]]:
    """Mirror for facing left around the pivot column (the runtime rule)."""
    return img.transpose(Image.Transpose.FLIP_LEFT_RIGHT), (img.width - 1 - pivot[0], pivot[1])


def up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def opaque_bbox(img: Image.Image) -> tuple[int, int, int, int] | None:
    """Inclusive ``(x0, y0, x1, y1)`` of the opaque pixels, or None for an empty frame."""
    box = img.getchannel("A").getbbox()
    return None if box is None else (box[0], box[1], box[2] - 1, box[3] - 1)


def font(size: int = 11) -> ImageFont.ImageFont:
    try:
        return ImageFont.load_default(size=size)
    except TypeError:     # Pillow without scalable default font
        return ImageFont.load_default()


def label(d: ImageDraw.ImageDraw, xy, text: str, fill=WHITE, size: int = 11) -> None:
    d.text(xy, text, fill=fill, font=font(size))


def text_width(text: str, size: int = 11) -> int:
    return int(font(size).getlength(text))


def cell(img: Image.Image, k: int, pivot, contact_row: int | None, marks: bool = True) -> Image.Image:
    """One frame at scale k on the review background; contact row cyan, pivot yellow."""
    base = Image.new("RGBA", img.size, CELL_BG + (255,))
    base.alpha_composite(img)
    big = up(base, k)
    if marks:
        d = ImageDraw.Draw(big)
        if contact_row is not None:
            y = contact_row * k + k - 1
            d.line([(0, y), (big.width, y)], fill=CYAN)
        d.rectangle([pivot[0] * k, pivot[1] * k, pivot[0] * k + k - 1, pivot[1] * k + k - 1], outline=YELLOW)
    return big


# ------------------------------------------------------------------ tick timing and GIFs

def gif_delays(ticks: list[int], hz: int = TICK_HZ) -> list[int]:
    """Whole-centisecond GIF delays (ms) from tick counts, rounded on the cumulative sum so no drift builds up."""
    out, cum, prev = [], 0, 0
    for t in ticks:
        cum += t
        cs = round(cum * 100 / hz)
        out.append(max(cs - prev, 1) * 10)
        prev = cs
    return out


def save_gif(images: list[Image.Image], delays: list[int], path: Path) -> None:
    pal = [im.convert("RGB").convert("P", palette=Image.Palette.ADAPTIVE, colors=64) for im in images]
    pal[0].save(path, save_all=True, append_images=pal[1:], duration=delays, loop=0, disposal=1, optimize=False)
