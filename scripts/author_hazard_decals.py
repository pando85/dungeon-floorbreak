#!/usr/bin/env python3
"""AREA-HAZARDS-01B: the acid entry splash, the corrosion decals and the Rot marks.

The design, the two frozen keys and the acceptance targets are in
``docs/02-art/area-hazards-01b-spec.md``. This module holds the **drawing** half: the
hand-authored row tables and the few procedural helpers that produce every frame of

- ``dfx_acid_splash`` (5 frames, frame 2 is the frozen key, loaded verbatim),
- ``dfx_corr_face`` (the frozen key, verbatim), ``dfx_corr_top`` and ``dfx_corr_wall``,
- ``dfx_rot_nest``, ``dfx_rot_trail`` (the ceiling nest and its track scuff) and
  ``dfx_rot_drip`` (the spent Rot Sticker's drip, ceiling and wall).

``scripts/author_danger_fx.py`` imports this module and writes the grids, the per-grid
configs and the manifests next to the other danger FX (``python3
scripts/author_danger_fx.py --meta``). No concept pixel is cropped, traced or resampled;
no image-model raster output. The script is deterministic: no randomness, no timestamps.

Palettes (spec sections 3.2, 3.3 and 5.1):

- the acid family (splash, corrosion) draws only the teal ``ACID_RAMP`` entries; the
  corrosion uses the **dry subset** (``acid_outline``, ``acid_0_deep``, ``acid_1_shade``,
  ``fume_dark``, ``fume_lit``), never a lit acid entry (HZB-AD4);
- the Rot marks draw only ROT-01 entries, read by name from ``rot_01.grid.json`` and
  never ``glow_*``, ``rib_red``, ``spike_tip`` or ``flash_white`` (HZB-AD5).

Every row table below uses the key characters of ``ACID_RAMP`` (the dfx_acid order) or the
ROT-01 grid keys, so a table reads exactly like the key grids it sits next to.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import author_gwkit_keys as K  # noqa: E402
from author_gwkit_keys import Canvas  # noqa: E402

ROOT = K.ROOT
KEY_DIR = ROOT / "assets/source/props/floor01/hz01b_keys"
ROT_GRID_REL = "assets/source/characters/rot_sticker/rot_01.grid.json"
ROT_GRID = ROOT / ROT_GRID_REL
SPEC = "docs/02-art/area-hazards-01b-spec.md"

# ---------------------------------------------------------------- palettes ====
# HZB-AD1: the teal acid ramp, in dfx_acid order (key characters are K.KEYS[i], so the
# ramp keeps the characters the SLIME-01 borrow had and every dfx_acid row is unchanged).
ACID_RAMP: tuple[tuple[str, str], ...] = (
    ("acid_outline", "#061418ff"),
    ("acid_0_deep", "#0d3034ff"),
    ("acid_1_shade", "#124c50ff"),
    ("acid_2_mid", "#197a7eff"),
    ("acid_3_light", "#2eb2a8ff"),
    ("acid_4_lit", "#78e2d4ff"),
    ("acid_sheen", "#c6f8eeff"),
    ("acid_fizz", "#eafffaff"),
    ("fume_lit", "#a4c4c0ff"),
    ("fume_dark", "#587a78ff"),
)
ACID_KEY = {name: K.KEYS[i] for i, (name, _rgba) in enumerate(ACID_RAMP)}
ACID_CHAR = {v: k for k, v in ACID_KEY.items()}
ACID_NAMES = tuple(n for n, _ in ACID_RAMP)
# The dry subset of the corrosion decals (spec 3.2): no lit acid, no sheen, no fizz.
DRY = frozenset({"acid_outline", "acid_0_deep", "acid_1_shade", "fume_dark", "fume_lit"})
# Never in a Rot mark (spec 3.3, HZB-AD5).
ROT_FORBIDDEN = frozenset({"glow_ember", "glow_hot", "glow_core", "rib_red", "spike_tip", "flash_white"})


def rot_palette() -> dict[str, str]:
    """The ROT-01 entries, name -> rgba, read from its grid (never retyped)."""
    return {p["name"]: p["rgba"] for p in json.loads(ROT_GRID.read_text())["palette"]}


def rot_chars() -> dict[str, str]:
    """The ROT-01 grid key characters, name -> key."""
    return {p["name"]: p["key"] for p in json.loads(ROT_GRID.read_text())["palette"]}


def rot_names() -> dict[str, str]:
    return {v: k for k, v in rot_chars().items()}


# --------------------------------------------------------------- row tables ====
def canvas_of(rows: list[str], names: dict[str, str]) -> Canvas:
    """A canvas from key-character rows (``.`` is transparent); ``names`` maps key -> entry."""
    w = len(rows[0])
    assert all(len(r) == w for r in rows), "ragged row table"
    cv = Canvas(w, len(rows))
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != ".":
                cv.set(x, y, names[ch])
    return cv


def acid_canvas(rows: list[str]) -> Canvas:
    return canvas_of(rows, ACID_CHAR)


def key_rows(name: str) -> list[str]:
    """The rows of a frozen key grid (one frame, layer ``body``), loaded verbatim (HZB-AD7)."""
    grid = json.loads((KEY_DIR / name).read_text())
    return list(grid["frames"][0]["cels"]["body"])


def blank(w: int, h: int) -> list[list[str]]:
    return [["."] * w for _ in range(h)]


def put(g: list[list[str]], x: int, y: int, s: str) -> None:
    """Write the characters of ``s`` at ``(x, y)``, clipped to the canvas; a space keeps the pixel."""
    for i, c in enumerate(s):
        if c != " " and 0 <= y < len(g) and 0 <= x + i < len(g[0]):
            g[y][x + i] = c


def rows_of(g: list[list[str]]) -> list[str]:
    return ["".join(r) for r in g]


# ================================================================== SPLASH ====
# 32 x 24, pivot (16, 23) on the patch's walking line; the pool surface is row 18, the
# displaced lip rows 16..18 and the trough rows 19..20 (spec 3.1). Drawn for a +x pop:
# the right sheet is the taller one and its drops fly toward +x; the presenter mirrors
# the whole frame about the pivot column for a -x pop. Every drop is a 2 px chunk.
SPLASH_W, SPLASH_H = 32, 24
SPLASH_TICKS = (2, 3, 3, 4, 4)  # 16 ticks, played once
KEY_SPLASH = "acid_splash_key.grid.json"


def splash_frames() -> list[Canvas]:
    key = key_rows(KEY_SPLASH)
    lip = key[16:21]  # the key's displaced lip and trough, rows 16..20

    def base() -> list[list[str]]:
        g = blank(SPLASH_W, SPLASH_H)
        for i, r in enumerate(lip):
            put(g, 0, 16 + i, r)
        return g

    # ---- frame 0 "dimple" (2 ticks): the surface dents, no sheets yet. A dark trough 8 px
    # wide on rows 17..19, a low lip of rows 16..17 on both sides (1 px higher on the +x
    # side) and two 2 px fizz beads above it. Rows 19..20 are the key's trough.
    f0 = blank(SPLASH_W, SPLASH_H)
    put(f0, 8, 13, "*")
    put(f0, 8, 14, "(")
    put(f0, 23, 12, "*")
    put(f0, 23, 13, "(")
    put(f0, 0, 15, ".....................%&'(.......")
    put(f0, 0, 16, "....(''&%$..........$%&''(......")
    put(f0, 0, 17, ".#('''&&&%%$$......$$%%&&''''(#.")
    put(f0, 0, 18, "#&&%%%$$$$$###$$$###$$$$%%%%&&&#")
    put(f0, 0, 19, key[19])
    put(f0, 0, 20, key[20])

    # ---- frame 1 "rise" (3 ticks): the key's sheets at about half height. The right tip
    # leans +x near row 9, the left tip near row 12. The lip as in the key. No detached drops.
    f1 = base()
    # right sheet: the key's rows 10..15 and a leaning tip on row 9
    for y in range(10, 16):
        put(f1, 0, y, "".join(c if 19 <= x <= 24 else " " for x, c in enumerate(key[y])))
    put(f1, 23, 9, "'(")
    put(f1, 22, 10, "'")  # (the key's row 10 already holds cols 21..23)
    # left sheet: the key's rows 12..15
    for y in range(12, 16):
        put(f1, 0, y, "".join(c if 8 <= x <= 14 else " " for x, c in enumerate(key[y])))

    # ---- frame 2 "crown" (KEY): verbatim.
    f2 = [list(r) for r in key]

    # ---- frame 3 "break" (4 ticks): the sheets collapse to stubs (right top near row 11,
    # left near row 13). Every drop of the key travels on along its arc, 2..3 px outward
    # (the right-hand ones clamp at the canvas edge): the high ones still rise 1 px, the
    # low ones fall 2 px. The wisp rises 2 px and turns fume_dark.
    f3 = base()
    for y in range(12, 16):
        put(f3, 0, y, "".join(c if 19 <= x <= 24 else " " for x, c in enumerate(key[y])))
    put(f3, 22, 11, "'(")
    for y in range(14, 16):
        put(f3, 0, y, "".join(c if 8 <= x <= 14 else " " for x, c in enumerate(key[y])))
    put(f3, 10, 13, "('")
    # left drops (outward is -x)
    for (x, y, c) in [(2, 3, "*"), (3, 4, "("), (3, 5, ")"), (3, 6, "("),  # the high strand, rising
                      (2, 12, "'"), (2, 13, "&"),                            # falling
                      (0, 15, "("), (0, 16, "'")]:                           # falling onto the lip
        put(f3, x, y, c)
    # right drops (outward is +x)
    for (x, y, c) in [(31, 0, "*"), (30, 1, "("),             # the top bead pair
                      (28, 0, "("), (27, 1, "'"),             # the second bead pair
                      (29, 2, ")"), (28, 3, "("), (27, 4, ")"), (26, 5, "'"), (27, 5, "("),  # the whip
                      (29, 7, "'"), (30, 6, "("),
                      (31, 12, "'"), (31, 13, "&")]:
        put(f3, x, y, c)
    # the wisp: up 2 px, dark
    for (x, y) in [(15, 9), (16, 10), (15, 11)]:
        put(f3, x, y, ",")

    # ---- frame 4 "settle" (4 ticks): no sheets. The lip sinks to rows 17..18 and its lit
    # ends move 2 px outward (the ripple). Three or four drops fall back onto rows 14..16.
    f4 = blank(SPLASH_W, SPLASH_H)
    put(f4, 0, 17, "#(''''&&%%$$")
    put(f4, 20, 17, "$$%%&&''''(#")
    put(f4, 0, 18, "#&&%%%%$$$$##....####$$$$%%%%&&#")
    put(f4, 1, 19, "#%%$$$##....$##$......##$$$%%#")
    put(f4, 2, 20, "###")
    put(f4, 27, 20, "###")
    for (x, y, c) in [(3, 15, "("), (3, 16, "'"),
                      (9, 14, "'"), (9, 15, "&"),
                      (22, 15, "("), (22, 16, "'"),
                      (27, 14, "'"), (27, 15, "&")]:
        put(f4, x, y, c)

    return [acid_canvas(rows_of(f)) for f in (f0, f1, f2, f3, f4)]


# =============================================================== CORROSION ====
# Dry decals (spec 3.2): old stains and eaten mortar, never lit, never liquid.
KEY_CORR = "corrosion_key.grid.json"

# 32 x 6, pivot (16, 5): a flat, blotchy stain on the floor-top band, the walking line on
# row 5. The darkest pixels sit in the middle; the ends fade out with no outline.
CORR_TOP_ROWS = [
    "................,%%,............",
    "........,%,..,%$$%%,.,%,........",
    "....,%%,.,%$$$$##$$%%,%$$%,.....",
    ".,%%$$%%%$$####$$$$$%$$$%%%,....",
    "..,%%$%%$$$##$$$#$$$$%$$%%,.,...",
    "....,,%%%%$$$$$$$%%%%%%%,,......",
]

# 12 x 24, pivot (0, 23): two or three thin, uneven runs climbing at most 20 px up the
# backwall from the floor line (rows 4..23), one eaten-mortar notch, never lit.
CORR_WALL_ROWS = [
    "............",  # 0
    "............",  # 1
    "............",  # 2
    "............",  # 3
    ".....,......",  # 4
    ".....%......",  # 5
    ".....%......",  # 6
    "....%%......",  # 7
    "..,..%......",  # 8
    "..%..%%.....",  # 9
    "..%...%.....",  # 10
    "..%...%.....",  # 11
    "..%%..%...,.",  # 12
    "...%..%%..%.",  # 13
    "...%...%..%.",  # 14
    "...%%..%..%.",  # 15
    "....%..%$#%.",  # 16
    "....%$.%$#%.",  # 17
    "....%$..%$%.",  # 18
    "...%$$..%$$.",  # 19
    "..%%$$$.%$$.",  # 20
    ".,%$$$$%%$$,",  # 21
    ",%%$$$$$$$%,",  # 22
    "%%$$##$$##%%",  # 23
]


def corr_top() -> Canvas:
    return acid_canvas(CORR_TOP_ROWS)


def corr_wall() -> Canvas:
    return acid_canvas(CORR_WALL_ROWS)


def corr_face() -> Canvas:
    """The floor-face sample: the frozen key, verbatim."""
    return acid_canvas(key_rows(KEY_CORR))


# ================================================================ ROT MARKS ====
# ROT-01 entries only (spec 3.3): the crust (outline_soot, shell_0..2), the gunk
# (rot_olive), one or two dead red flecks (rib_dead) and, only as the wet glint on a drip
# bead, rim_cold. The characters below are the ROT-01 grid keys, so a table reads like
# the sticker's own grid: K outline_soot, a shell_0_deep, b shell_1_shade, c shell_2_mid,
# v rim_cold, x rot_olive, m rib_dead.
NEST_W, NEST_H = 32, 12
TRAIL_W, TRAIL_H = 16, 3
DRIP_W, DRIP_H = 8, 16

# A crusted, star-cracked blotch on the ceiling's underside (built by ``_nest_rows``): 26 px wide
# at the top and tapering, four short gunk stalactites of 2..5 px. No dome, no rim highlight: it
# reads as "something lives here", not as a second sticker. Row 0 is the ceiling underside line.


def rot_canvas(rows: list[str]) -> Canvas:
    return canvas_of(rows, rot_names())


def _nest_rows() -> list[str]:
    """The ceiling nest: a faceted crust (cracks in soot, plates bevelled by the lit and
    shaded sides of each crack), a bitten underside, olive gunk in the cracks and four
    short stalactites."""
    spans = {0: (3, 28), 1: (3, 28), 2: (4, 27), 3: (5, 26), 4: (7, 24), 5: (9, 22), 6: (12, 19)}
    bites = [(5, 3), (24, 4), (10, 5), (21, 5)]
    g = blank(NEST_W, NEST_H)
    for y, (x0, x1) in spans.items():
        for x in range(x0, x1 + 1):
            if (x, y) not in bites:
                g[y][x] = "b"
    # the star cracks radiate from one core at (16, 2) and run out to the rim.
    cracks = [
        [(16, 1), (16, 2), (15, 3), (15, 4), (14, 5), (14, 6)],
        [(17, 2), (18, 3), (19, 3), (20, 4), (21, 5), (22, 5)],
        [(15, 2), (14, 2), (13, 1), (12, 1), (11, 2), (10, 2), (9, 3), (8, 3), (7, 4)],
        [(17, 1), (18, 1), (19, 2), (20, 2), (21, 1), (22, 1), (23, 2), (24, 2), (25, 3)],
        [(16, 3), (17, 4), (17, 5), (18, 6)],
    ]
    crack = {pt for line in cracks for pt in line}
    for x, y in crack:
        if g[y][x] != ".":
            g[y][x] = "K"
    # bevel: a plate pixel whose left or upper neighbour is a crack is lit (shell_2_mid), whose
    # right neighbour is a crack is in shade (shell_0_deep).
    plate = [row[:] for row in g]
    for y in range(1, NEST_H):
        for x in range(NEST_W):
            if plate[y][x] != "b":
                continue
            if plate[y][x - 1] == "K" or plate[y - 1][x] == "K":
                g[y][x] = "c"
            elif plate[y][x + 1] == "K" and y % 2 == 0:
                g[y][x] = "a"
    for x in range(3, 29):
        g[0][x] = "a"
    # soot rim along the underside (the line that separates the body from the brick).
    mask = {(x, y) for y, row in enumerate(g) for x, c in enumerate(row) if c != "."}
    for (x, y) in sorted(mask):
        if y >= 1 and (x, y + 1) not in mask and g[y][x] != "K":
            g[y][x] = "K"
        elif y >= 1 and ((x - 1, y) not in mask or (x + 1, y) not in mask):
            g[y][x] = "K"
    # gunk: olive pooling where the cracks reach the rim, then the stalactites.
    for x, y in [(14, 6), (14, 5), (18, 6), (22, 5), (9, 3), (8, 3), (24, 2), (25, 3)]:
        if g[y][x] != ".":
            g[y][x] = "x"
    stal = [(8, 5, 4), (13, 7, 5), (18, 7, 3), (22, 6, 3)]  # x, first row, length
    for x, y0, n in stal:
        for k in range(n):
            g[y0 + k][x] = "x" if k < n - 1 else "a"
        g[y0][x + 1] = "x"
        if n >= 4:
            g[y0 + 1][x + 1] = "x"
    # two dead red flecks echo the ribs (rib_dead), set in the crust.
    g[2][12] = "m"
    g[3][21] = "m"
    return rows_of(g)


def nest() -> Canvas:
    return rot_canvas(_nest_rows())


TRAIL_ROWS = [
    "..aa.x..aaa..xa.",
    ".a..xx..a..aa...",
    "........x.......",
]


def trail() -> Canvas:
    return rot_canvas(TRAIL_ROWS)


# ---- the spent drip: 8 x 16, ceiling pivot (4, 0); wall pivot (0, 0), authored for a
# WallLeft sticker (the presenter mirrors it for WallRight). ``drip_run_*`` plays once
# from the detonation tick (4 x 6 ticks), then ``drip_drop_*`` loops (3 x 20 ticks).
RUN_TICKS = 6
DROP_TICKS = 20


def _ceiling_drip(strand: int, bead_rows: tuple[int, ...] = (), splat: bool = True,
                  detached: tuple[int, ...] = ()) -> list[str]:
    """One ceiling drip frame: the splat on rows 0..1, a strand of ``strand`` rows (2 px wide
    for its first rows, 1 px below), an optional 2 x 2 bead (wet glint
    top left, shade bottom right) and an optional detached 2 px chunk."""
    g = blank(DRIP_W, DRIP_H)
    if splat:  # a 6 px olive splat on rows 0..1
        put(g, 1, 0, "xxxxxx")
        put(g, 2, 1, "xaxx")
    for y in range(2 if splat else 0, strand):
        g[y][4] = "x"
        if y < 4:
            g[y][3] = "x"
    for i, y in enumerate(bead_rows):  # a 2 px wide bead
        g[y][3] = "v" if i == 0 else "x"
        g[y][4] = "x" if i == 0 else "a"
    for i, y in enumerate(detached):  # the 2 px chunk that left the strand
        g[y][3] = "x"
        g[y][4] = "x" if i == 0 else "a"
    return rows_of(g)


def _wall_drip(strand: int, bead_rows: tuple[int, ...] = (), splat: bool = True,
               detached: tuple[int, ...] = ()) -> list[str]:
    """The same beats down a wall face (authored for ``WallLeft``: the face is column 0 and the
    room is to the right; the presenter mirrors it for ``WallRight``): a smear on rows 0..2 and
    a strand in column 1, two columns wide on its first rows."""
    g = blank(DRIP_W, DRIP_H)
    if splat:
        put(g, 0, 0, "xx")
        put(g, 0, 1, "xxx")
        put(g, 0, 2, "xa")
    for y in range(3 if splat else 0, strand):
        g[y][1] = "x"
        if y < 5:
            g[y][0] = "x"
    for i, y in enumerate(bead_rows):
        g[y][1] = "v" if i == 0 else "x"
        g[y][2] = "x" if i == 0 else "a"
    for i, y in enumerate(detached):
        g[y][1] = "x"
        g[y][2] = "x" if i == 0 else "a"
    return rows_of(g)


def drip_frames() -> dict[str, list[Canvas]]:
    """The four drip tags: run and drop, ceiling and wall."""
    out: dict[str, list[Canvas]] = {}
    for suffix, make in (("c", _ceiling_drip), ("w", _wall_drip)):
        run = [make(2), make(4), make(9, (7, 8)), make(13, (11, 12))]
        drop = [make(13, (13, 14)), make(12, (), True, (14, 15)), make(13)]
        out[f"drip_run_{suffix}"] = [rot_canvas(r) for r in run]
        out[f"drip_drop_{suffix}"] = [rot_canvas(r) for r in drop]
    return out


# ================================================================ SHEETS ====
Frame = tuple[Canvas, "int | None"]
TagSpec = tuple[str, list[Frame], str]

# Per grid: (tags, pivot). The wall drip variants share the ceiling pivot (4, 0) in the grid;
# the wall pivot (0, 0) is applied by the presenter (``src/hazard_decals.rs``).
ACID_FAMILY = ("dfx_acid", "dfx_acid_splash", "dfx_corr_face", "dfx_corr_top", "dfx_corr_wall")
ROT_FAMILY = ("dfx_rot_nest", "dfx_rot_trail", "dfx_rot_drip")
# The sheets this module draws (``dfx_acid`` is the existing pool, recoloured).
HAZARD_SHEETS = tuple(n for n in ACID_FAMILY + ROT_FAMILY if n != "dfx_acid")


def assets() -> dict[str, tuple[list[TagSpec], tuple[int, int]]]:
    drip = drip_frames()
    drip_tags: list[TagSpec] = []
    for suffix in ("c", "w"):
        drip_tags.append((f"drip_run_{suffix}", [(cv, RUN_TICKS) for cv in drip[f"drip_run_{suffix}"]], "once"))
        drip_tags.append((f"drip_drop_{suffix}", [(cv, DROP_TICKS) for cv in drip[f"drip_drop_{suffix}"]], "loop"))
    return {
        "dfx_acid_splash": ([("splash", list(zip(splash_frames(), SPLASH_TICKS)), "once")], (16, SPLASH_H - 1)),
        "dfx_corr_face": ([("corr_face", [(corr_face(), None)], "hold")], (16, 0)),
        "dfx_corr_top": ([("corr_top", [(corr_top(), None)], "hold")], (16, 5)),
        "dfx_corr_wall": ([("corr_wall", [(corr_wall(), None)], "hold")], (0, 23)),
        "dfx_rot_nest": ([("nest", [(nest(), None)], "hold")], (16, 0)),
        "dfx_rot_trail": ([("trail", [(trail(), None)], "hold")], (8, 0)),
        "dfx_rot_drip": (drip_tags, (4, 0)),
    }


# Frozen design keys (grid frame index): the splash crown and the floor-face corrosion sample.
FROZEN = {"dfx_acid_splash": [2], "dfx_corr_face": [0]}
TITLES = {
    "dfx_acid_splash": "acid entry splash (dimple, rise, crown, break, settle)",
    "dfx_corr_face": "corrosion, floor face (frozen key)",
    "dfx_corr_top": "corrosion, floor top stain",
    "dfx_corr_wall": "corrosion, backwall trail",
    "dfx_rot_nest": "Rot nest, ceiling",
    "dfx_rot_trail": "Rot track scuff",
    "dfx_rot_drip": "spent Rot Sticker drip (ceiling and wall, run and drop)",
}
# Evidence scene rows: (frame, x of the pivot column, lift) and whether a shadow is drawn.
SCENES = {
    "dfx_acid_splash": ([(0, 500, 0), (2, 580, 0), (4, 660, 0)], False),
    "dfx_corr_face": ([(0, 560, 0)], False),
    "dfx_corr_top": ([(0, 560, 0)], False),
    "dfx_corr_wall": ([(0, 560, 0)], False),
    "dfx_rot_nest": ([(0, 560, 120)], False),
    "dfx_rot_trail": ([(0, 560, 120)], False),
    "dfx_rot_drip": ([(0, 520, 120), (3, 600, 120), (7, 680, 120)], False),
}
STRIPS = {
    "dfx_acid_splash": ["splash"],
    "dfx_corr_face": ["corr_face"],
    "dfx_corr_top": ["corr_top"],
    "dfx_corr_wall": ["corr_wall"],
    "dfx_rot_nest": ["nest"],
    "dfx_rot_trail": ["trail"],
    "dfx_rot_drip": ["drip_run_c", "drip_drop_c", "drip_run_w", "drip_drop_w"],
}
# The sheets each key file feeds (the keys are inputs of the manifests).
KEY_FILES = {
    "dfx_acid_splash": [f"assets/source/props/floor01/hz01b_keys/{KEY_SPLASH}"],
    "dfx_corr_face": [f"assets/source/props/floor01/hz01b_keys/{KEY_CORR}"],
}


def key_map(name: str) -> dict[str, str] | None:
    """The fixed key characters of an acid-family or Rot grid (``None``: assign by order)."""
    if name in ACID_FAMILY:
        return ACID_KEY
    if name in ROT_FAMILY:
        return rot_chars()
    return None


def palette_id(name: str) -> str | None:
    if name in ACID_FAMILY:
        return "hz01b-acid-teal-v1"
    if name in ROT_FAMILY:
        return "rot-sticker-floor01-v1"
    return None
