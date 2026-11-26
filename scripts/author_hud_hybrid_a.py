#!/usr/bin/env python3
"""HUD-HYBRID-A-01 art execution: deterministic authoring record.

Builds, from text only (no concept pixels, no resampling):

* the per-asset ``pixel-grid-v1`` sources under ``assets/source/hud/``
  (``hud_portrait_carl``, ``hud_portrait_donut``, ``hud_system_crt``,
  ``hud_plate_frame``, ``hud_strip_3slice``, ``hud_notice_frame``, ``hud_glyphs``,
  ``hud_icons``, ``hud_tags``, ``hud_keycaps``, ``hud_chips``),
* the combined single-frame atlas grid ``hud_hybrid_a.grid.json`` (one layer per
  asset group) that ``scripts/export_assets.sh`` turns into ``hud_hybrid_a.ase``
  and the exported ``hud_hybrid_a`` atlas PNG plus metadata,
* the export manifest ``assets/manifests/hud_hybrid_a_v1.json`` (region rects and
  tool-independent SHA-256 of every region's RGBA pixels).

Design authority: the top-tier design spec (hybrid HUD, section 2, 3 and 7) and the two
frozen top-tier keys in ``assets/source/hud/keys/`` (Carl portrait, System CRT face).
The keys' pixels are reused unchanged in colour and position; only palette names,
pivot and accent aliases for deliberate 1 px details (lint level) are added.

Run: ``python3 scripts/author_hud_hybrid_a.py`` (rewrites the generated files).
``python3 scripts/author_hud_hybrid_a.py --check`` fails if committed files differ.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import pixel_grid_to_ase as pg  # noqa: E402

SRC = ROOT / "assets/source/hud"
KEYS = SRC / "keys"
MANIFEST = ROOT / "assets/manifests/hud_hybrid_a_v1.json"
DONUT_MASTER = ROOT / "assets/source/characters/donut/donut_h40.grid.json"
CARL_MASTER = ROOT / "assets/source/characters/carl/carl_h52.grid.json"

# --------------------------------------------------------------------------- palette
# Design spec section 3. Tokens drawn into the atlas only; bars, wells and flashes are
# code-drawn (listed in CODE_TOKENS and exported in the manifest for the renderer).
HUD = {
    "hud_ink": "#120c18", "plate_fill": "#211b27", "metal_dark": "#2a2430",
    "metal_mid": "#3d3542", "metal_light": "#5c5262", "metal_hi": "#8a7e8c",
    "rust": "#7a3f2a", "brass_dark": "#6a4a1c", "brass": "#b58a3a", "brass_hi": "#e8c063",
    "amber": "#ffc24a", "amber_dim": "#a8742a", "text_white": "#f4e6d2",
    "magenta_dk": "#6e1a4c", "magenta": "#e0338f", "magenta_hi": "#ff6fc0",
    "screen_bg": "#2a0d24", "crt_face_hi": "#ffb3dc", "notice_bg": "#1e0f22",
    "hp_red_lo": "#9a1a2c", "hp_red": "#d8283c", "hp_red_hi": "#ff5a5f",
    "mana_v0": "#7a3cf0", "cyan": "#5fe0e8",
}
CODE_TOKENS = {
    "hp_empty": "#3a1620", "hp_flash": "#fff1e0",
    "mana_v0": "#7a3cf0", "mana_v1": "#6a4ff0", "mana_v2": "#5a63f2", "mana_v3": "#4a7af4",
    "mana_v4": "#3d92f4", "mana_v5": "#33aaf0", "mana_v6": "#33c2ec", "mana_v7": "#45d8ea",
    "mana_empty": "#1c1a33", "well_carl": "#2b2038", "well_donut": "#301a2e",
}
# Accent aliases: same colour as their base token, flagged ``accent`` so a deliberate
# 1 px detail passes the strict orphan lint (pixel colour is unchanged).
ALIASES = {
    "rivet_hi": "brass_hi", "rivet_lo": "brass_dark", "notch": "plate_fill",
    "bang": "hud_ink", "spark_core": "cyan",
}
HUD_KEY = {  # stable single-character keys for the HUD namespace
    "hud_ink": "K", "plate_fill": "F", "metal_dark": "r", "metal_mid": "M",
    "metal_light": "L", "metal_hi": "H", "rust": "R", "brass_dark": "d", "brass": "B",
    "brass_hi": "b", "amber": "A", "amber_dim": "a", "text_white": "W",
    "magenta_dk": "e", "magenta": "m", "magenta_hi": "n", "screen_bg": "s",
    "crt_face_hi": "f", "notice_bg": "N", "hp_red_lo": "p", "hp_red": "P",
    "hp_red_hi": "q", "mana_v0": "v", "cyan": "c",
    "rivet_hi": "1", "rivet_lo": "2", "notch": "3", "bang": "4", "spark_core": "5",
}
assert len(set(HUD_KEY.values())) == len(HUD_KEY)


def hud_palette(rows: list[str]) -> list[dict]:
    used = {ch for r in rows for ch in r} - {"."}
    out = []
    for name, key in HUD_KEY.items():
        if key in used:
            base = ALIASES.get(name, name)
            entry = {"key": key, "name": name, "rgba": HUD[base] + "ff"}
            if name in ALIASES:
                entry["accent"] = True
            out.append(entry)
    return out


class Canvas:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.px = [["."] * w for _ in range(h)]

    def set(self, x: int, y: int, name: str) -> None:
        assert 0 <= x < self.w and 0 <= y < self.h, (x, y, name)
        self.px[y][x] = HUD_KEY[name]

    def rect(self, x: int, y: int, w: int, h: int, name: str) -> None:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                self.set(xx, yy, name)

    def hole(self, x: int, y: int, w: int, h: int) -> None:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                self.px[yy][xx] = "."

    def stamp(self, x: int, y: int, art: list[str], legend: dict[str, str]) -> None:
        for dy, row in enumerate(art):
            for dx, ch in enumerate(row):
                if ch != ".":
                    self.set(x + dx, y + dy, legend[ch])

    def rows(self) -> list[str]:
        return ["".join(r) for r in self.px]


def ring(c: Canvas, x: int, y: int, w: int, h: int) -> None:
    """3 px worn bevel ring (spec 2.1): ink | bevel | ink, fill is the caller's job."""
    for i, name in enumerate(("hud_ink", None, "hud_ink")):
        x0, y0, x1, y1 = x + i, y + i, x + w - 1 - i, y + h - 1 - i
        for xx in range(x0, x1 + 1):
            for yy in (y0, y1):
                c.set(xx, yy, name or ("metal_mid" if yy == y1 else "metal_hi"))
        for yy in range(y0, y1 + 1):
            for xx in (x0, x1):
                if name:
                    c.set(xx, yy, name)
                else:
                    # right column and bottom row are the shaded side of the bevel
                    c.set(xx, yy, "metal_mid" if (xx == x1 or yy == y1) else "metal_hi")


def bevel_ring_with_fill(w: int, h: int) -> Canvas:
    c = Canvas(w, h)
    c.rect(3, 3, w - 6, h - 6, "plate_fill")
    ring(c, 0, 0, w, h)
    return c


# --------------------------------------------------------------------------- pieces
def make_plate_frame() -> Canvas:
    c = bevel_ring_with_fill(164, 48)
    # portrait well (+4,+4,32,32): ink border, brass rim, transparent 28x28 interior
    c.rect(4, 4, 32, 32, "hud_ink")
    c.rect(5, 5, 30, 30, "brass")
    c.hole(6, 6, 28, 28)
    for (rx, ry) in ((4, 4), (34, 4), (4, 34), (34, 34)):
        c.set(rx, ry, "rivet_hi")
        c.set(rx + 1, ry, "brass")
        c.set(rx, ry + 1, "brass")
        c.set(rx + 1, ry + 1, "rivet_lo")
    # ownership tag area (+4,+36,32,9) left transparent for the tag sprite
    c.hole(4, 36, 32, 9)
    # wear: 5 rust clusters on the shaded bottom edge (row 46) and right edge (col 162),
    # all outside the text column x >= 40 on the bottom and clear of the content
    for x0, n in ((8, 2), (20, 3), (33, 2)):
        for dx in range(n):
            c.set(x0 + dx, 46, "rust")
    for y0, n in ((14, 2), (34, 3)):
        for dy in range(n):
            c.set(162, y0 + dy, "rust")
    return c


def make_notice_frame() -> Canvas:
    """HA2-AD3: the notice as a plate inside the same 204x32 rect.

    Ring 0 ``hud_ink``, ring 1 a worn bevel (``metal_hi`` top and left, ``metal_mid`` bottom
    and right, like the hero plates), ring 2 ``magenta`` (the System's colour), ring 3
    ``hud_ink``, fill ``notice_bg``; the dotted ``magenta_dk`` rule sits on row +26.
    """
    w, h = 204, 32
    c = Canvas(w, h)
    c.rect(4, 4, w - 8, h - 8, "notice_bg")
    for i, name in enumerate(("hud_ink", None, "magenta", "hud_ink")):
        x1, y1 = w - 1 - i, h - 1 - i
        for xx in range(i, x1 + 1):
            for yy in range(i, y1 + 1):
                if xx not in (i, x1) and yy not in (i, y1):
                    continue
                if name:
                    c.set(xx, yy, name)
                else:
                    # lit top row and left column; shaded right column and bottom row
                    c.set(xx, yy, "metal_hi" if (yy == i or xx == i) else "metal_mid")
    # dotted rule on row +26: 2-on / 2-off, x +4..+199 (2 px dashes pass the orphan lint)
    for xx in range(4, w - 4):
        if (xx // 2) % 2 == 0:
            c.set(xx, 26, "magenta_dk")
    return c


def strip_pieces() -> tuple[Canvas, dict[str, tuple[int, int, int, int]]]:
    """Location strip (16 px) and boss strip (26 px): caps 8 px, 1 px repeatable middle."""
    cv = Canvas(40, 26)
    rects: dict[str, tuple[int, int, int, int]] = {}

    def piece(name: str, ox: int, w: int, h: int, side: str) -> None:
        # full-width ring drawn on a wide scratch so caps and middle stay seamless
        wide = Canvas(24, h)
        wide.rect(3, 3, 18, h - 6, "plate_fill")
        ring(wide, 0, 0, 24, h)
        if side == "left":
            cols = range(0, 8)
        elif side == "mid":
            cols = range(10, 11)
        else:
            cols = range(16, 24)
        for i, sx in enumerate(cols):
            for yy in range(h):
                cv.px[yy][ox + i] = wide.px[yy][sx]
        rects[name] = (ox, 0, w, h)

    piece("strip_loc_left", 0, 8, 16, "left")
    piece("strip_loc_mid", 8, 1, 16, "mid")
    piece("strip_loc_right", 9, 8, 16, "right")
    piece("strip_boss_left", 20, 8, 26, "left")
    piece("strip_boss_mid", 28, 1, 26, "mid")
    piece("strip_boss_right", 29, 8, 26, "right")
    # magenta lamps in the location caps: 4x6 magenta_dk with a 2x4 magenta_hi core at cap x+4
    # (left cap) and cap x+0 (right cap, mirrored), y+5
    for ox in (4, 9):
        for yy in range(5, 11):
            for xx in range(ox, ox + 4):
                cv.set(xx, yy, "magenta_dk")
        for yy in range(6, 10):
            for xx in range(ox + 1, ox + 3):
                cv.set(xx, yy, "magenta_hi")
    return cv, rects


def disk(symbol: list[str] | None) -> Canvas:
    c = Canvas(11, 11)
    inside = {(x, y) for y in range(11) for x in range(11)
              if (x - 5) ** 2 + (y - 5) ** 2 <= 30}
    for (x, y) in inside:
        edge = any((x + dx, y + dy) not in inside for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        c.set(x, y, "hud_ink" if edge else ("metal_light" if y <= 5 else "metal_mid"))
    if symbol:
        c.stamp(3, 3, symbol, {"W": "text_white"})
    return c


SYMBOLS = {
    "glyph_a": [".WWW.", "W...W", "WWWWW", "W...W", "W...W"],
    "glyph_b": ["WWWW.", "W...W", "WWWW.", "W...W", "WWWW."],
    "glyph_x": ["W...W", ".W.W.", "..W..", ".W.W.", "W...W"],
    "glyph_y": ["W...W", ".W.W.", "..W..", "..W..", "..W.."],
    "glyph_cross": ["W...W", "WW.WW", ".WWW.", "WW.WW", "W...W"],
    "glyph_circle": [".WWW.", "W...W", "W...W", "W...W", ".WWW."],
    "glyph_square": ["WWWWW", "W...W", "W...W", "W...W", "WWWWW"],
    "glyph_triangle": ["..W..", ".W.W.", ".W.W.", "W...W", "WWWWW"],
}


def socket_locked() -> Canvas:
    c = Canvas(11, 11)
    c.rect(0, 0, 11, 11, "hud_ink")
    c.rect(1, 1, 9, 9, "metal_dark")
    c.stamp(2, 2, ["..LLL..", ".L...L.", ".L...L.", "LLLLLLL", "LLLKLLL", "LLLKLLL", "LLLLLLL"],
            {"L": "metal_light", "K": "hud_ink"})
    return c


def heart(cracked: bool) -> Canvas:
    c = Canvas(7, 7)
    c.stamp(0, 0, [".XX.XX.", "XXXXXXX", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X..."],
            {"X": "hp_red"})
    c.set(1, 0, "hp_red_hi")
    c.set(1, 1, "hp_red_hi")
    for (x, y) in ((5, 3), (4, 4), (3, 5)):
        c.set(x, y, "hp_red_lo")
    if cracked:
        for (x, y) in ((3, 1), (2, 2), (3, 3), (2, 4)):
            c.set(x, y, "hud_ink")
    return c


def spark(depleted: bool) -> Canvas:
    c = Canvas(5, 5)
    if not depleted:
        c.stamp(0, 0, ["..V..", ".VVV.", "VVCVV", ".VVV.", "..V.."], {"V": "mana_v0", "C": "spark_core"})
    else:
        c.stamp(0, 0, ["..H..", ".H.H.", "H...H", ".H.H.", "..H.."], {"H": "metal_hi"})
        for (x, y) in ((0, 4), (1, 3), (2, 2), (3, 1), (4, 0)):
            c.set(x, y, "amber_dim")
    return c


def warning() -> Canvas:
    c = Canvas(11, 10)
    widths = (1, 3, 3, 5, 5, 7, 7, 9, 9, 11)
    for y, w in enumerate(widths):
        c.rect(5 - w // 2, y, w, 1, "amber")
    for y in (3, 4, 5):
        c.set(5, y, "hud_ink")
    c.set(5, 7, "bang")
    return c


def boss_emblem() -> Canvas:
    c = Canvas(20, 20)
    c.rect(0, 0, 20, 20, "hud_ink")
    c.rect(1, 1, 18, 18, "magenta_dk")
    c.stamp(5, 6, ["..WWWWWW..", ".WWWWWWWW.", "WWWWWWWWWW", "WWKKWWKKWW", "WWKKWWKKWW",
                   ".WWWKKWWW.", "..WWWWWW..", "..W.WW.W.."], {"W": "text_white", "K": "hud_ink"})
    return c


def tag(kind: str) -> Canvas:
    c = Canvas(32, 9)
    if kind == "p1":
        c.rect(0, 0, 32, 9, "amber")
    elif kind == "p2":
        c.rect(0, 0, 32, 9, "magenta_hi")
        for (x, y) in ((0, 0), (31, 0), (0, 8), (31, 8)):
            c.set(x, y, "notch")
    else:
        c.rect(0, 0, 32, 9, "cyan")
        c.rect(1, 1, 30, 7, "hud_ink")
    return c


# --------------------------------------------------------------------------- HUD-ART-02 key caps and chips
# Spec hud-art-02-spec.md section 2.1: square 11x11 key cap, 1 px cut corners, lit bevel (top, left),
# a metal_light face, a metal_mid bottom lip and a two-row metal_dark skirt. ``key_j`` and
# ``key_arrow_right`` are the frozen design keys (copied pixel-exact and tested); the other caps are
# drawn here from the construction and the 5x5 symbols of section 2. ``key_space`` is the same cap
# stretched to 19 px (every row keeps its caps; the face, bevel and skirt widen).
CAP_BASE = [
    ".KKKKKKKKK.",
    "KHHHHHHHHMK",
    "KHLLLLLLLMK",
    "KHLLLLLLLMK",
    "KHLLLLLLLMK",
    "KHLLLLLLLMK",
    "KHLLLLLLLMK",
    "KMMMMMMMMMK",
    "KrrrrrrrrrK",
    "KrrrrrrrrrK",
    ".KKKKKKKKK.",
]
CAP_LEGEND = {"K": "hud_ink", "H": "metal_hi", "L": "metal_light", "M": "metal_mid", "r": "metal_dark"}
CAP_SYMBOLS = {  # 5x5 at cols 3..7, rows 2..6 of the cap; W is text_white
    "key_k": ["W...W", "W..W.", "WWW..", "W..W.", "W...W"],
    "key_l": ["W....", "W....", "W....", "W....", "WWWWW"],
    "key_q": [".WWW.", "W...W", "W.W.W", "W..W.", ".WW.W"],
    "key_left": ["..W..", ".W...", "WWWWW", ".W...", "..W.."],
    "key_down": ["..W..", "..W..", "W.W.W", ".WWW.", "..W.."],
}
CAP_SPACE_SYMBOL = ["W.......W", "WWWWWWWWW"]  # 9x2 open bracket at cols 5..13, rows 5..6 of the wide cap
# Frozen design keys the executor copies (cap tag in the key file -> our region name).
CAP_KEYS = {"key_j": "key_j", "key_arrow_right": "key_right"}
CAP_ORDER = ["key_j", "key_k", "key_l", "key_q", "key_space", "key_left", "key_right", "key_down"]
CHIP_ORDER = ["chip_dynamite", "chip_puddle"]
CAP_W = {"key_space": 19}


def cap(symbol: list[str] | None, wide: int = 11) -> Canvas:
    base = [r[:5] + r[5] * (wide - 10) + r[6:] for r in CAP_BASE] if wide > 11 else list(CAP_BASE)
    c = Canvas(wide, 11)
    c.stamp(0, 0, base, CAP_LEGEND)
    if symbol:
        x0, y0 = (3, 2) if wide == 11 else (5, 5)
        c.stamp(x0, y0, symbol, {"W": "text_white"})
    return c


def key_canvas(doc: dict, frame: int) -> Canvas:
    """A frozen key frame as a HUD-namespace canvas (palette names and colours are verified)."""
    by_key = {e["key"]: e for e in doc["palette"]}
    rows = key_flat(doc, frame)
    c = Canvas(len(rows[0]), len(rows))
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch == ".":
                continue
            e = by_key[ch]
            assert HUD[e["name"]] + "ff" == e["rgba"], e
            c.set(x, y, e["name"])
    return c


def key_tag_frames(doc: dict) -> dict[str, int]:
    return {t["name"]: t["from"] for t in doc["tags"]}


def cap_frames() -> list[tuple[str, Canvas]]:
    keys = load(KEYS / "hud_art_02_keycaps.grid.json")
    tags = key_tag_frames(keys)
    frames: dict[str, Canvas] = {}
    for key_tag, name in CAP_KEYS.items():
        frames[name] = key_canvas(keys, tags[key_tag])
    for name, sym in CAP_SYMBOLS.items():
        frames[name] = cap(sym)
    frames["key_space"] = cap(CAP_SPACE_SYMBOL, 19)
    return [(n, frames[n]) for n in CAP_ORDER]


def chip_frames() -> list[tuple[str, Canvas]]:
    keys = load(KEYS / "hud_art_02_chips.grid.json")
    tags = key_tag_frames(keys)
    return [(n, key_canvas(keys, tags[n])) for n in CHIP_ORDER]


def detail_grid(frames: list[tuple[str, Canvas]], w: int, h: int, spare: str) -> dict:
    """Multi-frame source; lone 1 px details get same-colour accent aliases (lint level only)."""
    rows = [pad_bottom(c, w, h) for _, c in frames]
    pal = hud_palette([r for f in rows for r in f])
    rows, pal = detail_aliases(rows, pal, spare)
    return grid_doc((w, h), pal, {"art": rows}, tag_table([n for n, _ in frames]))


# --------------------------------------------------------------------------- grid I/O
def grid_doc(canvas: tuple[int, int], palette: list[dict], layers: dict[str, list[list[str]]],
             tags: list[dict] | None = None, pivot_y: int | None = None) -> dict:
    """Frames: ``layers`` maps layer name -> list of per-frame row lists."""
    names = list(layers)
    nframes = len(layers[names[0]])
    frames = [{"duration_ms": 100, "cels": {n: layers[n][i] for n in names}} for i in range(nframes)]
    doc = {"format": "pixel-grid-v1", "canvas": list(canvas), "pivot": [0, canvas[1] - 1 if pivot_y is None else pivot_y],
           "palette": palette, "layers": names, "frames": frames, "tags": tags or []}
    return doc


def bottom_opaque(rows_per_frame: list[list[str]]) -> int:
    bottoms = [max(y for y, r in enumerate(rows) if set(r) - {"."}) for rows in rows_per_frame]
    assert len(set(bottoms)) == 1, bottoms
    return bottoms[0]


def dump(doc: dict) -> str:
    return json.dumps(doc, indent=1, ensure_ascii=False) + "\n"


def tag_table(names: list[str]) -> list[dict]:
    return [{"name": n, "from": i, "to": i, "direction": "forward"} for i, n in enumerate(names)]


def pad_bottom(c: Canvas, w: int, h: int) -> list[str]:
    """Place ``c`` bottom-left on a w x h canvas (equal bottom row across a frame set)."""
    out = Canvas(w, h)
    for y in range(c.h):
        for x in range(c.w):
            out.px[h - c.h + y][x] = c.px[y][x]
    return out.rows()


def multi_grid(frames: list[tuple[str, Canvas]], w: int, h: int) -> dict:
    rows = [pad_bottom(c, w, h) for _, c in frames]
    allrows = [r for f in rows for r in f]
    return grid_doc((w, h), hud_palette(allrows), {"art": rows}, tag_table([n for n, _ in frames]))


def single_grid(c: Canvas) -> dict:
    rows = c.rows()
    return grid_doc((c.w, c.h), hud_palette(rows), {"art": [rows]}, pivot_y=bottom_opaque([rows]))


# --------------------------------------------------------------------------- keys
def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def key_flat(doc: dict, fi: int) -> list[str]:
    return ["".join(r) for r in pg.composite_keys(pg.validate_grid(doc), fi)]


def detail_aliases(rows_by_frame: list[list[str]], palette: list[dict], spare: str) -> tuple[list[list[str]], list[dict]]:
    """Re-key lone 1 px orphans to same-colour accent aliases (lint level only)."""
    accents = {e["key"] for e in palette if e.get("accent")}
    by_key = {e["key"]: e for e in palette}
    spare_iter = iter(spare)
    alias_for: dict[str, str] = {}
    new_rows = []
    for rows in rows_by_frame:
        flat = [list(r) for r in rows]
        orphans, isolated = pg._stray_pixels([list(r) for r in rows], accents)
        assert not isolated, isolated
        for x, y in orphans:
            k = rows[y][x]
            if k not in alias_for:
                alias_for[k] = next(spare_iter)
            flat[y][x] = alias_for[k]
        new_rows.append(["".join(r) for r in flat])
    pal = list(palette)
    for k, a in alias_for.items():
        pal.append({"key": a, "name": by_key[k]["name"] + "_detail", "rgba": by_key[k]["rgba"], "accent": True})
    return new_rows, pal


# --------------------------------------------------------------------------- AUDIENCE-01 viewer icons
# Top-tier pixel keys (design review of PR #118, section 4), copied pixel-exact. Every glyph is its own
# single-frame source (glyphs are not animation frames). Digits and arrows are one-colour masks that
# the renderer tints per state: digits text_white (untinted = stable, amber by tint), up arrow
# magenta_hi, down arrow amber (tinted to amber_dim by default, untinted = the high-contrast amber).
# The decimal point is NOT art: it is a code-placed 1x1 rect (a lone pixel is not a sprite, and the
# strict lint rejects isolated pixels).
VIEWER_EYE = [".........", "..BBBBB..", ".B.HHH.B.", "B.HHkHH.B", ".B.HHH.B.", "..BBBBB..", "........."]
VIEWER_UP = ["..X..", ".XXX.", "XXXXX", "..X..", "..X..", "..X..", "..X.."]
VIEWER_DOWN = ["..X..", "..X..", "..X..", "..X..", "XXXXX", ".XXX.", "..X.."]
VIEWER_DIGITS = {
    "0": ".XXX. X...X X..XX X.X.X XX..X X...X .XXX.",
    "1": "..X.. .XX.. ..X.. ..X.. ..X.. ..X.. .XXX.",
    "2": ".XXX. X...X ....X ...X. ..X.. .X... XXXXX",
    "3": "XXXXX ...X. ..X.. ...X. ....X X...X .XXX.",
    "4": "...X. ..XX. .X.X. X..X. XXXXX ...X. ...X.",
    "5": "XXXXX X.... XXXX. ....X ....X X...X .XXX.",
    "6": "..XX. .X... X.... XXXX. X...X X...X .XXX.",
    "7": "XXXXX ....X ...X. ..X.. .X... .X... .X...",
    "8": ".XXX. X...X X...X .XXX. X...X X...X .XXX.",
    "9": ".XXX. X...X X...X .XXXX ....X ...X. .XX..",
    "k": "X...X X..X. X.X.. XX... X.X.. X..X. X...X",
    "m": "X...X XX.XX X.X.X X.X.X X...X X...X X...X",
    "plus": "..... ..X.. ..X.. XXXXX ..X.. ..X.. .....",
}
VIEWER_DIGIT_ORDER = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "k", "m", "plus"]
VIEWER_SOURCES = ["hud_viewer_eye", "hud_viewer_up", "hud_viewer_down"] + [
    f"hud_viewer_digit_{d}" for d in VIEWER_DIGIT_ORDER]


def viewer_canvas(art: list[str], legend: dict[str, str]) -> Canvas:
    c = Canvas(len(art[0]), len(art))
    c.stamp(0, 0, art, legend)
    return c


def viewer_grid(c: Canvas) -> dict:
    """Single-frame source; a lone 1 px detail (the eye pupil) gets a same-colour accent alias."""
    rows = c.rows()
    pal = hud_palette(rows)
    [rows], pal = detail_aliases([rows], pal, "!#$%&*")
    return grid_doc((c.w, c.h), pal, {"art": [rows]}, pivot_y=bottom_opaque([rows]))


def viewer_sources() -> dict[str, dict]:
    docs = {
        "hud_viewer_eye": viewer_grid(viewer_canvas(VIEWER_EYE, {"B": "brass", "H": "brass_hi", "k": "hud_ink"})),
        "hud_viewer_up": viewer_grid(viewer_canvas(VIEWER_UP, {"X": "magenta_hi"})),
        "hud_viewer_down": viewer_grid(viewer_canvas(VIEWER_DOWN, {"X": "amber"})),
    }
    for d in VIEWER_DIGIT_ORDER:
        docs[f"hud_viewer_digit_{d}"] = viewer_grid(viewer_canvas(VIEWER_DIGITS[d].split(), {"X": "text_white"}))
    return docs


def make_portrait_carl() -> dict:
    key = load(KEYS / "carl_portrait.grid.json")
    rows = [key_flat(key, 0)]
    pal = key["palette"]
    carl_master = {e["rgba"] for e in load(CARL_MASTER)["palette"]}
    assert all(e["rgba"] in carl_master for e in pal), "key palette must be carl_h52 colours"
    rows, pal = detail_aliases(rows, pal, "!#$%&*+,-/:;<=>?@^_~|")
    pal = [dict(e, name="carl_" + e["name"]) for e in pal]
    return grid_doc((28, 28), pal, {"portrait": rows}, tag_table(["portrait"]),
                    pivot_y=bottom_opaque(rows))


CRT_RENAME = {"metal_light": "metal_hi", "magenta_hi": "crt_face_hi"}


def make_system_crt() -> dict:
    key = load(KEYS / "system_crt_face.grid.json")
    by_key = {e["key"]: e for e in key["palette"]}
    frames = []
    for fi in range(3):
        frames.append(key_flat(key, fi))
    # remap key chars to the HUD namespace by token name (spec names), verifying colours
    char_map: dict[str, str] = {}
    for k, e in by_key.items():
        name = CRT_RENAME.get(e["name"], e["name"])
        assert HUD[name] + "ff" == e["rgba"], (name, e)
        char_map[k] = HUD_KEY[name]
    frames = [["".join(char_map.get(ch, ch) for ch in r) for r in f] for f in frames]
    # orphans -> accent aliases of the same HUD colour (the CRT teeth, pupils, sparkles)
    accents: set[str] = set()
    alias: dict[str, str] = {}
    spare = iter("xyzXYZ!#$%&*")
    out_frames = []
    for rows in frames:
        flat = [list(r) for r in rows]
        orphans, isolated = pg._stray_pixels([list(r) for r in rows], accents)
        assert not isolated
        for x, y in orphans:
            k = rows[y][x]
            if k not in alias:
                alias[k] = next(spare)
            flat[y][x] = alias[k]
        out_frames.append(["".join(r) for r in flat])
    inv = {v: n for n, v in HUD_KEY.items()}
    pal = hud_palette([r for f in out_frames for r in f if True])
    have = {e["key"] for e in pal}
    for k, a in alias.items():
        pal.append({"key": a, "name": inv[k] + "_detail", "rgba": HUD[inv[k]] + "ff", "accent": True})
    pal = [e for e in pal if e["key"] in have or e["key"] in alias.values()]
    return grid_doc((20, 18), pal, {"art": out_frames}, tag_table(["idle", "blink", "notice"]),
                    pivot_y=bottom_opaque(out_frames))


def make_portrait_donut() -> dict:
    master = load(DONUT_MASTER)
    g = pg.validate_grid(master)
    flat = [list(r) for r in pg.composite_keys(g, 0)]
    x0, y0 = 14, 4  # head crop, 1:1 density, crown inside row 0, nose on the centre column pair
    rows = ["".join(flat[y][x0:x0 + 28]) for y in range(y0, y0 + 28)]
    # edge repaint where the crop leaves lone pixels (explicit, per the spec's cut-edge rule)
    fixes = {(0, 21): "O", (27, 21): "o", (10, 27): "C", (23, 27): "C"}
    grid = [list(r) for r in rows]
    for (x, y), ch in fixes.items():
        grid[y][x] = ch
    rows = ["".join(r) for r in grid]
    used = {ch for r in rows for ch in r} - {"."}
    pal = [dict(e, name="donut_" + e["name"]) for e in master["palette"] if e["key"] in used]
    return grid_doc((28, 28), pal, {"portrait": [rows]}, tag_table(["portrait"]),
                    pivot_y=bottom_opaque([rows]))


# --------------------------------------------------------------------------- atlas
CHARSET = ("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
           "!#$%&*+,-/:;<=>?@^_~|()[]{}")

ATLAS_W, ATLAS_H = 208, 168


def regions_plan() -> list[tuple[str, str, int, tuple[int, int]]]:
    """(atlas_name, source_asset, source_frame, (x, y)) in atlas pixel coordinates."""
    plan = [("plate_frame", "hud_plate_frame", 0, (0, 0)),
            ("portrait_carl", "hud_portrait_carl", 0, (172, 0)),
            ("portrait_donut", "hud_portrait_donut", 0, (172, 30)),
            ("notice_frame", "hud_notice_frame", 0, (0, 60)),
            ("crt_idle", "hud_system_crt", 0, (0, 94)),
            ("crt_blink", "hud_system_crt", 1, (22, 94)),
            ("crt_notice", "hud_system_crt", 2, (44, 94)),
            ("tag_p1", "hud_tags", 0, (70, 94)),
            ("tag_p2", "hud_tags", 1, (104, 94)),
            ("tag_ai", "hud_tags", 2, (138, 94)),
            ("boss_emblem", "hud_icons", 5, (174, 94)),
            ("warning", "hud_icons", 4, (196, 94))]
    for i, n in enumerate(("glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle",
                           "glyph_square", "glyph_triangle", "socket_locked")):
        plan.append((n, "hud_glyphs", i, (i * 13, 116)))
    plan += [("heart", "hud_icons", 0, (120, 116)), ("heart_cracked", "hud_icons", 1, (129, 116)),
             ("spark", "hud_icons", 2, (138, 116)), ("spark_depleted", "hud_icons", 3, (145, 116))]
    for n, x, y in (("strip_loc_left", 0, 130), ("strip_loc_mid", 8, 130), ("strip_loc_right", 9, 130),
                    ("strip_boss_left", 24, 130), ("strip_boss_mid", 32, 130), ("strip_boss_right", 33, 130)):
        plan.append((n, "hud_strip_3slice", 0, (x, y)))
    # AUDIENCE-01 part A: eye and trend arrows, then 13 digit masks (stride 6, 1 px gap)
    plan += [("viewer_eye", "hud_viewer_eye", 0, (48, 130)), ("viewer_up", "hud_viewer_up", 0, (60, 130)),
             ("viewer_down", "hud_viewer_down", 0, (67, 130))]
    for i, d in enumerate(VIEWER_DIGIT_ORDER):
        plan.append((f"viewer_digit_{d}", f"hud_viewer_digit_{d}", 0, (48 + i * 6, 140)))
    # HUD-ART-02: eight key caps and two item chips in one row under the strips (the atlas grew by 12 rows;
    # every existing region keeps its name, size and position)
    x = 0
    for i, n in enumerate(CAP_ORDER):
        plan.append((n, "hud_keycaps", i, (x, 157)))
        x += CAP_W.get(n, 11) + 1
    for i, n in enumerate(CHIP_ORDER):
        plan.append((n, "hud_chips", i, (x, 157)))
        x += 12
    return plan


# natural (w, h) of each region and its offset inside its source frame
SOURCE_RECT = {
    "plate_frame": (0, 0, 164, 48), "portrait_carl": (0, 0, 28, 28), "portrait_donut": (0, 0, 28, 28),
    "notice_frame": (0, 0, 204, 32), "crt_idle": (0, 0, 20, 18), "crt_blink": (0, 0, 20, 18),
    "crt_notice": (0, 0, 20, 18), "tag_p1": (0, 0, 32, 9), "tag_p2": (0, 0, 32, 9), "tag_ai": (0, 0, 32, 9),
    "boss_emblem": (0, 0, 20, 20), "warning": (0, 10, 11, 10),
    "heart": (0, 13, 7, 7), "heart_cracked": (0, 13, 7, 7), "spark": (0, 15, 5, 5),
    "spark_depleted": (0, 15, 5, 5),
    "strip_loc_left": (0, 0, 8, 16), "strip_loc_mid": (8, 0, 1, 16), "strip_loc_right": (9, 0, 8, 16),
    "strip_boss_left": (20, 0, 8, 26), "strip_boss_mid": (28, 0, 1, 26), "strip_boss_right": (29, 0, 8, 26),
}
SOURCE_RECT["viewer_eye"] = (0, 0, 9, 7)
for _n in ["viewer_up", "viewer_down"] + [f"viewer_digit_{d}" for d in VIEWER_DIGIT_ORDER]:
    SOURCE_RECT[_n] = (0, 0, 5, 7)
for _n in CAP_ORDER:
    SOURCE_RECT[_n] = (0, 0, CAP_W.get(_n, 11), 11)
for _n in CHIP_ORDER:
    SOURCE_RECT[_n] = (0, 0, 11, 11)
for _n in ("glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle", "glyph_square",
           "glyph_triangle", "socket_locked"):
    SOURCE_RECT[_n] = (0, 0, 11, 11)

LAYER_OF = {
    "hud_plate_frame": "plate", "hud_portrait_carl": "portraits", "hud_portrait_donut": "portraits",
    "hud_notice_frame": "notice", "hud_system_crt": "crt", "hud_tags": "tags",
    "hud_icons": "icons", "hud_glyphs": "glyphs", "hud_strip_3slice": "strip",
    "hud_keycaps": "keycaps", "hud_chips": "chips",
    **{n: "viewers" for n in VIEWER_SOURCES},
}


def build_sources() -> dict[str, dict]:
    docs: dict[str, dict] = {}
    docs["hud_portrait_carl"] = make_portrait_carl()
    docs["hud_portrait_donut"] = make_portrait_donut()
    docs["hud_system_crt"] = make_system_crt()
    docs["hud_plate_frame"] = single_grid(make_plate_frame())
    strip_cv, _ = strip_pieces()
    docs["hud_strip_3slice"] = single_grid(strip_cv)
    docs["hud_notice_frame"] = single_grid(make_notice_frame())
    order = ["glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle", "glyph_square",
             "glyph_triangle"]
    glyph_frames = [(n, disk(SYMBOLS[n])) for n in order] + [("socket_locked", socket_locked())]
    docs["hud_glyphs"] = multi_grid(glyph_frames, 11, 11)
    icon_frames = [("heart", heart(False)), ("heart_cracked", heart(True)), ("spark", spark(False)),
                   ("spark_depleted", spark(True)), ("warning", warning()), ("boss_emblem", boss_emblem())]
    docs["hud_icons"] = multi_grid(icon_frames, 20, 20)
    docs.update(viewer_sources())
    docs["hud_tags"] = multi_grid([("p1", tag("p1")), ("p2", tag("p2")), ("ai", tag("ai"))], 32, 9)
    docs["hud_keycaps"] = detail_grid(cap_frames(), 19, 11, "!#$%&*")
    docs["hud_chips"] = detail_grid(chip_frames(), 11, 11, "!#$%&*")
    return docs


def source_pixels(docs: dict[str, dict], asset: str, frame: int) -> list[list[tuple[str, str] | None]]:
    """Frame composite as (palette name, rgba) per pixel, None = transparent."""
    g = pg.validate_grid(docs[asset])
    flat = pg.composite_keys(g, frame)
    pal = {e["key"]: (e["name"], e["rgba"]) for e in g["palette"]}
    return [[None if ch == "." else pal[ch] for ch in row] for row in flat]


def build_atlas(docs: dict[str, dict]) -> tuple[dict, list[dict]]:
    # combined palette: merge by (name, rgba, accent) and assign deterministic keys
    entries: dict[tuple, str] = {}
    palette: list[dict] = []
    charset = iter(CHARSET)
    layers = {n: [["."] * ATLAS_W for _ in range(ATLAS_H)] for n in
              ("plate", "portraits", "notice", "crt", "tags", "icons", "glyphs", "strip", "viewers",
               "keycaps", "chips")}
    regions = []
    for name, asset, frame, (ax, ay) in regions_plan():
        sx, sy, w, h = SOURCE_RECT[name]
        pix = source_pixels(docs, asset, frame)
        accent_names = {e["name"] for e in docs[asset]["palette"] if e.get("accent")}
        layer = layers[LAYER_OF[asset]]
        rgba = bytearray()
        for yy in range(h):
            for xx in range(w):
                cell = pix[sy + yy][sx + xx]
                if cell is None:
                    rgba += b"\x00\x00\x00\x00"
                    continue
                nm, hexv = cell
                acc = nm in accent_names
                sig = (nm, hexv, acc)
                if sig not in entries:
                    entries[sig] = next(charset)
                    pe = {"key": entries[sig], "name": nm, "rgba": hexv}
                    if acc:
                        pe["accent"] = True
                    palette.append(pe)
                layer[ay + yy][ax + xx] = entries[sig]
                rgba += bytes.fromhex(hexv[1:])
        regions.append({"name": name, "source": asset, "source_frame": frame, "rect": [ax, ay, w, h],
                        "sha256": hashlib.sha256(bytes(rgba)).hexdigest()})
    layer_rows = {n: [["".join(r) for r in rows]] for n, rows in layers.items()}
    doc = grid_doc((ATLAS_W, ATLAS_H), palette, layer_rows, [], pivot_y=None)
    bottom = bottom_opaque([["".join(r) for r in _composite(layers)]])
    doc["pivot"] = [0, bottom]
    return doc, regions


def _composite(layers: dict[str, list[list[str]]]) -> list[list[str]]:
    out = [["."] * ATLAS_W for _ in range(ATLAS_H)]
    for rows in layers.values():
        for y, r in enumerate(rows):
            for x, ch in enumerate(r):
                if ch != ".":
                    out[y][x] = ch
    return out


def file_sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest_doc(regions: list[dict], atlas_doc: dict, docs: dict[str, dict]) -> dict:
    keys = {k: file_sha(KEYS / f"{k}.grid.json") for k in ("carl_portrait", "system_crt_face", "hud_art_02_keycaps", "hud_art_02_chips",
                                          "hud_art_02_notice_plate")}
    return {
        "schema_version": 1,
        "id": "hud_hybrid_a_v1",
        "type": "decor_atlas",
        "source_path": "assets/source/hud/hud_hybrid_a.ase",
        "grid_source": "assets/source/hud/hud_hybrid_a.grid.json",
        "output_atlas": "generated/art/hud_hybrid_a.png",
        "output_metadata": "generated/art/hud_hybrid_a.json",
        "palette_id": "hud-hybrid-a-v1",
        "frame_size": [ATLAS_W, ATLAS_H],
        "pivot": atlas_doc["pivot"],
        "animation_tags": [],
        "required_alpha": True,
        "owner": "AI-authored HUD art (HUD-HYBRID-A-01, hybrid A HUD: plates, strips, notice frame, glyphs, "
                 "icons, tags, hero portraits, System CRT face; AUDIENCE-01 viewer eye, trend arrows and 5x7 digit masks; "
                 "HUD-ART-02 keyboard key caps, item chips and the notice backing plate)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; "
                   "no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Derived from approved sprite masters and top-tier keys; no AI concept pixels copied. "
                      "Carl portrait and System CRT face are the frozen top-tier keys (assets/source/hud/keys/), "
                      "pixels unchanged, lone 1 px details re-keyed to same-colour accent aliases for the strict "
                      "lint. Donut portrait is a 28x28 1:1 crop of assets/source/characters/donut/donut_h40.grid.json "
                      "frame 0 (x 14..41, y 4..31) with four edge pixels repainted. Frames, strips, notice frame, "
                      "glyphs, icons and tags are original pixel-grid-v1 art written by scripts/author_hud_hybrid_a.py "
                      "from the design spec (sections 2, 3, 7). The AUDIENCE-01 viewer eye, arrows and digits are "
                      "copied pixel-exact from the top-tier keys in the PR #118 design review. HUD-ART-02: key_j, key_right, "
                      "chip_dynamite and chip_puddle are the frozen top-tier keys (assets/source/hud/keys/hud_art_02_*), "
                      "copied pixel-exact (lone 1 px details re-keyed to same-colour accent aliases for the strict lint); "
                      "key_k, key_l, key_q, key_space, key_left and key_down are drawn from the spec's cap construction "
                      "and 5x5 symbols; notice_frame is the HA2-AD3 plate, equal to the frozen notice-plate key. "
                      "The HUD concept PNGs were never sampled, traced or cropped.",
            "generator": "Design and keys: Claude Opus. Execution (portrait crop, frames, glyphs, icons, tags, atlas, "
                         "validators, evidence): Claude Sonnet via scripts/author_hud_hybrid_a.py. "
                         "Independent review: pending, a separate top-tier agent. No image-model raster output.",
            "generated_art": True,
            "input_reference_ids": ["assets/source/characters/donut/donut_h40.grid.json",
                                    "assets/source/characters/carl/carl_h52.grid.json (palette of the Carl key)"],
            "key_sha256": keys,
        },
        "hud": {
            "atlas_size": [ATLAS_W, ATLAS_H],
            "regions": regions,
            "frame_counts": {"hud_portrait_carl": 1, "hud_portrait_donut": 1, "hud_system_crt": 3,
                             "hud_glyphs": 9, "hud_tags": 3, "hud_icons": 6, "hud_keycaps": 8, "hud_chips": 2},
            "portrait_palettes": {"portrait_carl": "carl_h52 subset (names carl_*)",
                                  "portrait_donut": "donut_h40 subset (names donut_*)"},
            "hud_tokens": dict(sorted(HUD.items())),
            "code_drawn_tokens": CODE_TOKENS,
            "notes": [
                "portrait wells: plate_frame leaves the 28x28 interior at atlas (rect.x+6, rect.y+6) transparent; "
                "the renderer fills it with code_drawn_tokens.well_carl / well_donut under the portrait",
                "tag area (+4,+36,32,9) of plate_frame is transparent; draw tag_p1 / tag_p2 / tag_ai there",
                "strip_*: draw left cap, repeat the 1 px mid column to the wanted width, draw right cap",
                "glyph names a, b, x, y are the Xbox set, cross/circle/square/triangle the DualSense set",
                "viewer_*: AUDIENCE-01 part A, single-frame sources hud_viewer_*; eye 9x7 (brass, brass_hi, "
                "hud_ink pupil), up 5x7 magenta_hi, down 5x7 amber (renderer tints amber_dim, untinted in "
                "high contrast), digits 5x7 text_white masks (advance 6; K and M and + included)",
                "key_*: HUD-ART-02 keyboard key caps (11x11, key_space 19x11), HUD tokens only, a lit metal bevel "
                "with cut corners and a two-row metal_dark skirt; the renderer draws the cap for the keyboard "
                "glyph set in the same place as a pad glyph. chip_*: 11x11 item chips for the grant notice well "
                "(chip_dynamite on well_carl, chip_puddle on well_donut), drawn at notice +184,+10",
                "notice_frame: HA2-AD3 plate (ink, metal bevel, magenta, ink ring, magenta_dk rule on row +26); "
                "the 15x15 item well (hud_ink, brass ring, well_carl or well_donut fill) at notice +182,+8 is "
                "code-drawn, not in the atlas",
                "the decimal point is not in the atlas: the renderer draws it as a code-placed 1x1 rect on "
                "digit row 6 with advance 2 (a lone pixel is not a sprite and fails the strict lint)",
            ],
        },
    }


def write_all(check: bool) -> int:
    docs = build_sources()
    atlas_doc, regions = build_atlas(docs)
    outputs: dict[Path, str] = {}
    for name, doc in docs.items():
        pg.validate_grid(doc)
        outputs[SRC / f"{name}.grid.json"] = dump(doc)
    outputs[SRC / "hud_hybrid_a.grid.json"] = dump(atlas_doc)
    # manifest needs the committed key files (hash), which are inputs, not outputs
    outputs[MANIFEST] = json.dumps(manifest_doc(regions, atlas_doc, docs), indent=2, ensure_ascii=False) + "\n"
    bad = 0
    for path, text in outputs.items():
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"DIFFERS: {path.relative_to(ROOT)}", file=sys.stderr)
                bad = 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    return bad


if __name__ == "__main__":
    raise SystemExit(write_all("--check" in sys.argv[1:]))
