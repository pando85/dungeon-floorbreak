#!/usr/bin/env python3
"""Review renders for DOZ-02 (Dozer damage states and the stop frame): the design keys and the execution frames.

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases; the Bevy captures come
from ``scripts/make_dozer_doz02_evidence.sh``):

    python3 scripts/make_dozer_doz02_review.py [--out DIR] [--reversal-frames DIR]

Reads the DOZ-01 and DOZ-02 pixel-grid sources and the two rig contracts (``dozer_rig_h120.json``,
``dozer_damage_h120.json``) and writes deterministically into DIR (default docs/evidence/doz-02):

- ``doz02_contact_x1.png`` / ``doz02_contact_x4.png``: every DOZ-02 frame at native size and at x4;
- ``doz02_frames_x6.png``: every execution frame at x6 with its pivot marked and its tick count;
- ``doz02_key_sheet_x4.png``: the design keys at x4 beside their DOZ-01 originals (the gate pack's
  enlarged key sheet);
- ``doz02_lit_before_640x360.png`` / ``doz02_lit_after_640x360.png`` / ``doz02_lit_pair_x2.png``: the lit Floor 1
  corridor at gameplay scale (A1 compositor): the DOZ-01 machine under the raised frame with Carl at the brace,
  then the disabled machine (jammed wheel, bent yoke, slumped driver, the riders on the floor) under the landed
  remnant, the staging chosen so that no wall torch burns behind the cab or the bar;
- ``doz02_state_diagram.png``: the prop, machine and room-piece states with their frames and ticks;
- ``doz02_glance_x1.gif`` / ``doz02_glance_x3.gif``: the rear glance composed tick by tick from the atlases (the
  room has no late-release pilot, so there is no Bevy capture of it);
- ``doz02_reversal_x1.gif`` / ``doz02_reversal_x3.gif``: the Bevy reversal captures of
  ``make_dozer_doz02_evidence.sh`` (``--reversal-frames``, one frame per 60 Hz tick) as 2 cs GIFs; skipped when
  the frames are not given;
- ``doz02_bevy_contact.png`` / ``doz02_bevy_crops_x2.png``: the contact sheet and the x2 crops of the Bevy stills;
- ``SHA256SUMS`` of every file here except itself and the README.

Composed PNGs and GIFs, not Bevy captures, except the reversal GIFs and the ``doz02_bevy_*`` stills.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

import author_dozer_doz01 as ad  # noqa: E402
import author_dozer_doz02 as a2  # noqa: E402
import make_goblin_gob01_review as gr  # noqa: E402

DOZER = "assets/source/enemies/dozer"
PROPS = "assets/source/props/stop_frame"
CARL = "assets/source/characters/carl/carl_h52_locomotion.grid.json"
DEFAULT_OUT = ROOT / "docs" / "evidence" / "doz-02"
BG = (24, 23, 34)
WHITE, GREY, YELLOW = (236, 232, 240), (150, 146, 170), (255, 214, 0)
GROUND_LIGHT, GROUND_DARK = (88, 80, 96), (52, 46, 60)

# Lit corridor staging (layout px). The corridor ceiling is 276 px above the walking line, the room 3 lane
# ceiling 228 px: the frame hangs at its room height (mass bottom 168 px up) and the pulley block sits 48 px
# below the corridor ceiling.
# The staging keeps every light of the corridor off the machine (silhouette gate note G-4): the wall torches burn at
# view x 111 and 455 and 590 and a row of distant lamps glows at x 269, 292, 313, 357 and 401 (y 203). With the rig
# origin O at view x 255 the cab, the boiler and the stack have only brick behind them, the lamps sit behind the
# tub and the wheel disc, and the bar's upper end (rig x 168) has no lamp near it.
MACHINE_O_X = 315          # rig origin O + 60, layout px from the view's left edge
FRAME_X = 393              # stop-frame mass rectangle left edge, view x (over the tub and the wheel)
FRAME_LIFT = 168
BRACE_X = 533              # brace target left edge, view x
CARL_X = 525               # Carl in front of the brace
WHEEL_FRONT = 168          # the wheel front, rig px from O (the simulation's wheel_front)
LAND_DX = (32, 72)         # where the dismounted riders land, px past the wheel front (DZ-Q20)


def grid(path: str):
    return gr.grid_frames(path)


def frame_of(path: str, tag: str) -> tuple[Image.Image, tuple[int, int]]:
    frames, piv, g = grid(path)
    t = next(t for t in g["tags"] if t["name"] == tag)
    return frames[t["from"]], piv


def rider(tag: str) -> tuple[Image.Image, tuple[int, int]]:
    frames, piv, g = grid(f"{DOZER}/dozer_riders_h36.grid.json")
    t = next(t for t in g["tags"] if t["name"] == tag)
    return frames[t["from"]], piv


STATES = ("intact", "pitch_0", "pitch_1", "disabled")


def compose(state: str) -> tuple[Image.Image, tuple[int, int]]:
    """The assembled machine (pivot = rig origin O) for one state. ``intact``: the DOZ-01 rig, riders seated;
    ``pitch_0`` / ``pitch_1``: the impact keys, riders lurching forward on their pitched seats; ``disabled``:
    level, the driver slumped, the riders thrown out (the lit scene puts them on the floor)."""
    rig = json.loads((ROOT / DOZER / "dozer_rig_h120.json").read_text())
    anchors = {p["part"]: tuple(p["anchor"]) for p in rig["parts"] if isinstance(p.get("anchor"), list)}
    dmg = state != "intact"
    amp = a2.PITCH_AMPS.get(state, 0.0)
    seat_dy = {n: a2.pitch_dy(anchors[n][0], amp) for n in ("engineer", "rider_a", "rider_b")}
    parts = []

    def add(z, img_piv, anchor, dy=0):
        img, piv = img_piv
        parts.append((z, img, piv, (anchor[0], anchor[1] + dy)))
    hull_tag = state if dmg else "body"
    add(1, frame_of(f"{DOZER}/dozer_hull_damage_h120.grid.json" if dmg else f"{DOZER}/dozer_hull_h120.grid.json",
                    hull_tag), anchors["hull_body"])
    add(2, frame_of(f"{DOZER}/dozer_tread_h120.grid.json", "belt_0"), anchors["tread"])
    if state == "intact":
        tags = ad.RIDER_SEATED
    elif state == "disabled":
        tags = {}
    else:
        tags = {"engineer": "engineer_lurch_forward", "rider_a": "rider_a_lurch_forward",
                "rider_b": "rider_b_lurch_forward"}
    for name, tag in tags.items():
        add(4 if name == "rider_b" else 3, rider(tag), anchors[name], seat_dy[name])
    if state == "disabled":
        add(3, frame_of(f"{DOZER}/dozer_engineer_slump_h120.grid.json", "engineer_slump_0"), anchors["engineer"])
    for name in ("front_cab", "front_tub"):
        if dmg:
            add(5, frame_of(f"{DOZER}/dozer_{name}_damage_h120.grid.json", state), anchors[f"hull_{name}"])
        else:
            add(5, frame_of(f"{DOZER}/dozer_{name}_h120.grid.json", name), anchors[f"hull_{name}"])
    add(6, frame_of(f"{DOZER}/dozer_wheel_jammed_h120.grid.json", "jammed") if dmg
        else frame_of(f"{DOZER}/dozer_wheel_h120.grid.json", "phase_0"), anchors["wheel"])
    add(7, frame_of(f"{DOZER}/dozer_yoke_damage_h120.grid.json", "bent") if dmg
        else frame_of(f"{DOZER}/dozer_yoke_h120.grid.json", "static"), anchors["yoke"])
    W, H = 380, 184
    ox, fy = 96, H - 1
    canvas = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    for _z, img, piv, (ax, ay) in sorted(parts, key=lambda q: q[0]):
        canvas.alpha_composite(img, (ox + ax - piv[0], fy - ay - piv[1]))
    return canvas, (ox, fy)


def up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.NEAREST)


def panel(items, k: int, title: str, pad: int = 8) -> Image.Image:
    """Images (img, caption) in one row on a floor line at scale k, captioned."""
    ims = [up(i, k) for i, _c in items]
    w = max(pad + sum(i.width + pad for i in ims), pad * 2 + 6 * len(title))
    h = 18 + max(i.height for i in ims) + 18
    out = Image.new("RGBA", (w, h), BG + (255,))
    d = ImageDraw.Draw(out)
    d.text((pad, 3), title, fill=YELLOW, font=gr.font())
    x = pad
    base = 18 + max(i.height for i in ims)
    for im, (_i, cap) in zip(ims, items):
        out.alpha_composite(im, (x, base - im.height))
        d.text((x, base + 3), cap, fill=WHITE, font=gr.font())
        x += im.width + pad
    return out


def stack(images, gap: int = 6) -> Image.Image:
    w = max(i.width for i in images)
    h = sum(i.height for i in images) + gap * (len(images) - 1)
    out = Image.new("RGBA", (w, h), BG + (255,))
    y = 0
    for i in images:
        out.alpha_composite(i, (0, y))
        y += i.height + gap
    return out


def crop_opaque(img: Image.Image, pad: int = 1) -> Image.Image:
    x0, y0, x1, y1 = img.getchannel("A").getbbox()
    return img.crop((max(0, x0 - pad), max(0, y0 - pad), min(img.width, x1 + pad), min(img.height, y1 + pad)))


def contact(k: int) -> Image.Image:
    rows = []
    for stem in a2.STEMS:
        folder = a2.STEMS[stem][2].relative_to(ROOT).as_posix()
        frames, _piv, g = grid(f"{folder}/{stem}.grid.json")
        names = [t["name"] for t in g["tags"] if t["from"] == t["to"]]
        rows.append(panel(list(zip(frames, names)), k, stem))
    return stack(rows)


def key_sheet() -> Image.Image:
    k = 4
    live, _ = frame_of(f"{DOZER}/dozer_wheel_h120.grid.json", "phase_0")
    jam, _ = frame_of(f"{DOZER}/dozer_wheel_jammed_h120.grid.json", "jammed")
    yoke0, _ = frame_of(f"{DOZER}/dozer_yoke_h120.grid.json", "static")
    yoke1, _ = frame_of(f"{DOZER}/dozer_yoke_damage_h120.grid.json", "bent")
    raised, _ = frame_of(f"{PROPS}/prop_stop_frame.grid.json", "raised")
    pulley, _ = frame_of(f"{PROPS}/prop_stop_frame.grid.json", "pulley")
    hang = raised.copy()
    hang.alpha_composite(pulley)
    rem0, _ = frame_of(f"{PROPS}/prop_stop_frame_remnant.grid.json", "landed")
    rem1, _ = frame_of(f"{PROPS}/prop_stop_frame_remnant.grid.json", "landed_hit")
    brace, _ = frame_of(f"{PROPS}/prop_stop_frame_brace.grid.json", "intact")
    machines = {s: crop_opaque(compose(s)[0]) for s in STATES}
    return stack([
        panel([(live, "DOZ-01 live wheel (phase_0)"), (jam, "DOZ-02 jammed"), (yoke0, "DOZ-01 yoke"),
               (yoke1, "DOZ-02 yoke bent")], k, "item 1: the wheel (x4)"),
        panel([(machines["intact"], "DOZ-01 intact"), (machines["disabled"], "DOZ-02 disabled")], 2,
              "item 3: the disabled machine (x2)"),
        panel([(machines["pitch_0"], "pitch_0 (impact key)"), (machines["pitch_1"], "pitch_1")], 2,
              "item 2: the impact pitch (x2)"),
        panel([(hang, "raised + pulley"), (rem0, "remnant landed"), (rem1, "remnant landed_hit"),
               (brace, "brace intact")], k, "item 4: the stop frame (x4)"),
    ])


HAUL_BOTTOM = 49            # the haul tiles' bottom edge above the hanging mass' bottom (the presenter's rule)
HAUL_STEP = 8


def haul_entries(frame_x: int, brace_x: int) -> list[tuple[Image.Image, tuple[int, int], int, int]]:
    """The haul line as ``Floor.render`` entries (pivot (0, 11), the tile's bottom row on the walking line less
    ``lift``): ``h`` tiles from the pulley frame's right edge to the idler over the brace chain, the idler, then
    ``v`` tiles down to the brace top: the presenter's placement (``src/dozer_art.rs``, ``haul_sprites``)."""
    tiles = {t: frame_of(f"{PROPS.replace('stop_frame', 'dozer_room')}/env_dozer_haul.grid.json", t)[0]
             for t in ("h", "v", "idler")}
    piv = (0, 11)
    row_lift = FRAME_LIFT + HAUL_BOTTOM
    chain_x = brace_x + 8
    idler_left = chain_x - 6
    out = []
    x = frame_x - 8 + a2.FRAME_CANVAS[0]
    while x < idler_left:
        out.append((tiles["h"], piv, x, row_lift))
        x += HAUL_STEP
    out.append((tiles["idler"], piv, idler_left, row_lift))
    top = row_lift
    while top > 64:
        out.append((tiles["v"], piv, chain_x - 5, top - 12))
        top -= HAUL_STEP
    return out


def rows_img(rows) -> Image.Image:
    pal = {k: gr.pg.parse_rgba(c) for k, _n, c, _a in ad.ALL_PALETTE}
    img = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
    px = img.load()
    for y, r in enumerate(rows):
        for x, k in enumerate(r):
            if k != ".":
                px[x, y] = pal[k]
    return img


def smoke(tag: str) -> tuple[Image.Image, tuple[int, int]]:
    return frame_of(f"{DOZER}/dozer_smoke.grid.json", tag)


def lit(state: str) -> Image.Image:
    floor = gr.Floor()
    vx = floor.view[0]
    machine, mp = compose(state)
    o_x = vx + MACHINE_O_X - 60
    raised, rp = frame_of(f"{PROPS}/prop_stop_frame.grid.json", "raised")
    pulley, _ = frame_of(f"{PROPS}/prop_stop_frame.grid.json", "pulley")
    brace, bp = frame_of(f"{PROPS}/prop_stop_frame_brace.grid.json", "intact" if state == "intact" else "snapped")
    entries = [(brace, bp, vx + BRACE_X, 0)]
    entries += haul_entries(vx + FRAME_X, vx + BRACE_X)
    entries.append((pulley, rp, vx + FRAME_X, FRAME_LIFT))
    if state == "intact":
        entries.append((raised, rp, vx + FRAME_X, FRAME_LIFT))
        entries.append((machine, mp, o_x, 0))
    else:
        dmg = json.loads((ROOT / DOZER / "dozer_damage_h120.json").read_text())
        leak = dmg["emitters"]["steam_leak"]
        stack_e = dmg["emitters"]["stack_crooked"]
        machine = machine.copy()                          # the emitters' puffs ride on the machine sprite
        for tag, (ex, ey), (dx, dy) in (("steam_1", leak, (3, 5)), ("black_5", stack_e, (-4, 10))):
            puff, pp = smoke(tag)
            machine.alpha_composite(puff, (mp[0] + ex + dx - pp[0], mp[1] - ey - dy - pp[1]))
        entries.append((machine, mp, o_x, 0))
        rem, remp = frame_of(f"{PROPS}/prop_stop_frame_remnant.grid.json", "landed_hit")
        entries.append((rem, remp, vx + FRAME_X, 0))
        # the crew the simulation drops on the disable tick: rider A on his back, rider B getting up (GOB-01)
        lying, lp = frame_of(f"{DOZER}/dozer_rider_tumble_h120.grid.json", "rider_a_tumble_2")
        entries.append((lying, lp, o_x + WHEEL_FRONT + LAND_DX[0], 0))
        gframes, gpiv, _g = grid("assets/source/characters/goblin/goblin_b_h36.grid.json")
        entries.append((gframes[23], gpiv, o_x + WHEEL_FRONT + LAND_DX[1], 0))
    carl, cp = frame_of(CARL, "idle")
    entries.append((carl, cp, vx + CARL_X, 0))
    return floor.render(entries, 1)


EXEC_STEMS = ("prop_stop_frame", "prop_stop_frame_shadow", "prop_stop_frame_glance", "prop_stop_frame_brace",
              "env_dozer_gate", "env_dozer_buffer", "env_dozer_lintel", "env_dozer_haul",
              "dozer_rider_tumble_h120", "dozer_engineer_slump_h120")
EXEC_TAGS = {   # the tags drawn in the execution step (the design keys are not repeated here)
    "prop_stop_frame": ("warn_0", "warn_1", "falling"), "prop_stop_frame_brace": ("weakened", "snapped", "snapped_swing"),
}


def frames_x6(width: int = 1900) -> Image.Image:
    """Every execution frame at x6, its pivot marked (a yellow cross) and its tick count labelled; rows wrap at
    ``width``."""
    k = 6
    cells = []
    for stem in EXEC_STEMS:
        folder = a2.STEMS[stem][2].relative_to(ROOT).as_posix()
        frames, piv, g = grid(f"{folder}/{stem}.grid.json")
        names = [t["name"] for t in g["tags"] if t["from"] == t["to"]]
        ticks = a2.FRAME_TICKS[stem]
        for i, (img, name) in enumerate(zip(frames, names)):
            if stem in EXEC_TAGS and name not in EXEC_TAGS[stem]:
                continue
            t = ticks[i]
            cap = f"{stem.replace('dozer_', '').replace('prop_stop_frame', 'frame')}: {name}  " + \
                  (f"{t} ticks" if t else "held")
            big = up(img.convert("RGBA"), k)
            tile = Image.new("RGBA", (max(big.width, 8 * len(cap) + 4), big.height + 22), BG + (255,))
            tile.alpha_composite(big, (0, 0))
            d = ImageDraw.Draw(tile)
            cx, cy = piv[0] * k + k // 2, piv[1] * k + k // 2
            d.line([(cx - 7, cy), (cx + 7, cy)], fill=YELLOW)
            d.line([(cx, cy - 7), (cx, cy + 7)], fill=YELLOW)
            d.rectangle([0, 0, big.width - 1, big.height - 1], outline=GREY)
            d.text((2, big.height + 5), cap, fill=WHITE, font=gr.font())
            cells.append(tile)
    rows, row, x = [], [], 0
    for c in cells:
        if row and x + c.width > width:
            rows.append(row)
            row, x = [], 0
        row.append(c)
        x += c.width + 10
    rows.append(row)
    sheet_h = sum(max(c.height for c in r) + 10 for r in rows) + 10
    sheet = Image.new("RGBA", (width, sheet_h), BG + (255,))
    y = 10
    for r in rows:
        x = 10
        for c in r:
            sheet.alpha_composite(c, (x, y))
            x += c.width + 10
        y += max(c.height for c in r) + 10
    return sheet


def state_diagram() -> Image.Image:
    """The frame and state diagram: the prop states with their frames and ticks, the machine's states and their
    DOZ-02 art, the miss branches and the room pieces."""
    W, H = 1500, 1010
    img = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(img)
    f = gr.font()

    def title(y, text):
        d.text((10, y), text, fill=YELLOW, font=f)

    def box(x, y, w, h, lines, thumb=None, outline=GREY, k=2):
        d.rectangle([x, y, x + w, y + h], outline=outline)
        ty = y + 4
        if thumb is not None:
            t = up(thumb.convert("RGBA"), k)
            img.alpha_composite(t, (x + (w - t.width) // 2, y + 4))
            ty = y + 8 + t.height
        for ln in lines:
            d.text((x + 5, ty), ln, fill=WHITE, font=f)
            ty += 12

    def arrow(x0, y0, x1, y1, text=""):
        d.line([(x0, y0), (x1, y1)], fill=GREY)
        d.polygon([(x1, y1), (x1 - 6 * (1 if x1 > x0 else -1), y1 - 3), (x1 - 6 * (1 if x1 > x0 else -1), y1 + 3)],
                  fill=GREY)
        if text:
            d.text(((x0 + x1) // 2 - 4 * len(text) // 2, y0 - 12), text, fill=CYAN_T, font=f)

    def fr(stem, tag):
        folder = a2.STEMS[stem][2].relative_to(ROOT).as_posix() if stem in a2.STEMS else DOZER
        return frame_of(f"{folder}/{stem}.grid.json", tag)[0]

    def machine(state):
        return crop_opaque(compose(state)[0])

    # --- the stop frame
    title(6, "stop frame (prop): Supported -> Weakened -> Released -> Falling -> Landed   [60 Hz ticks]")
    sf = "prop_stop_frame"
    cell = [("Supported", fr(sf, "raised"), ["raised + pulley", "brace: intact", "held"]),
            ("Weakened (1 of 2 hits)", fr(sf, "raised"), ["raised + pulley", "brace: weakened", "held"]),
            ("Released: 30 ticks", fr(sf, "warn_0"), ["warn_0 / warn_1 every 3", "shadow s0..s3 grows", "brace: snapped (swings)"]),
            ("Falling: 8 ticks", fr(sf, "falling"), ["falling", "shadow s3", "pulley stays up"]),
            ("Landed: hit", fr("prop_stop_frame_remnant", "landed_hit"), ["landed_hit (bar in wheel)", "dust at both ends", "held"]),
            ("Landed: miss", fr("prop_stop_frame_remnant", "landed"), ["landed (early miss / glance)", "dust at both ends", "held"])]
    x = 10
    for n, (name, im, lines) in enumerate(cell):
        w = 232
        box(x, 24, w, 138, [name] + lines, im if im.width <= 100 else im.resize((im.width // 2, im.height // 2)),
            k=1 if im.width > 100 else 1)
        if n < 4:
            arrow(x + w, 90, x + w + 10, 90)
        x += w + 12
    d.text((10, 168), "Landed: hit when the front collider takes it (machine -> impact_stunned); miss when it lands ahead or "
           "behind (early remnant); glance when the hull is under the footprint while moving (below).", fill=GREY, font=f)
    # --- the machine
    title(190, "machine (Dozer): committed_chase -> impact_stunned (48 ticks) -> disabled; the miss branches")
    boxes = [("committed_chase", machine("intact"), ["DOZ-01 live parts", "wheel spins, tread rolls"]),
             ("impact_stunned 0..5", machine("pitch_0"), ["pitch_0; jammed wheel, bent yoke", "spark on the tub lip; crew lurches"]),
             ("6..11", machine("pitch_1"), ["pitch_1; steam every 6 ticks", "A tumbles from 12, B from 16"]),
             ("12..47", machine("disabled"), ["disabled hull level; driver slumps", "riders land (tumble_2)"]),
             ("disabled (terminal)", machine("disabled"), ["thin steam /40, wisp /60", "goblins get up (simulation, tick 48)"])]
    x = 10
    for n, (name, im, lines) in enumerate(boxes):
        w = 282
        box(x, 208, w, 200, [name] + lines, im, k=1, outline=GREY)
        x += w + 10
        if n < 4:
            arrow(x - 10, 300, x, 300)
    d.text((10, 414), "miss branches (machine unharmed): early remnant (frame lands ahead: landed); rear glance (a late landing meets "
           "only the rear hull): the frame keeps its live parts", fill=GREY, font=f)
    gl = "prop_stop_frame_glance"
    x = 10
    for tag, cap in (("glance_0", "glance_0: 4 ticks, spark"), ("glance_1", "glance_1: 4, -15 deg, 6 back"),
                     ("glance_2", "glance_2: 6, -35 deg, 16 back 8 down"), ("shadow", "shadow under 0 and 1")):
        im = fr(gl, tag)
        box(x, 432, 232, 120, [cap], im.crop(im.getchannel("A").getbbox()) if tag != "shadow" else im.crop((12, 82, 76, 88)), k=1)
        x += 242
    arrow(10 + 232, 490, 10 + 242, 490)
    arrow(10 + 484, 490, 10 + 494, 490)
    box(x, 432, 232, 120, ["then: the remnant `landed`", "drops in behind the machine,", "dust at both ends"],
        fr("prop_stop_frame_remnant", "landed"), k=1)
    # --- the room pieces
    title(570, "room pieces")
    gate = "env_dozer_gate"
    x = 10
    for tag, cap in (("closed", ["dormant, before the rumble"]), ("shake_0", ["approaching: shake_0 /", "shake_1 every 3 ticks"]),
                     ("shake_1", [""]), ("burst", ["from the emergence on"])):
        box(x, 588, 150, 170, [f"gate: {tag}"] + cap, fr(gate, tag), k=2)
        x += 160
    box(x, 588, 150, 170, ["buffer: idle", "drawn only (DZ-AD4)"], fr("env_dozer_buffer", "idle"), k=2)
    x += 160
    for tag in ("mouth", "fill"):
        box(x, 588, 150, 170, [f"lintel: {tag}", "not wired (kit lintel)" if tag == "mouth" else ""], fr("env_dozer_lintel", tag), k=2)
        x += 160
    x = 10
    for tag, cap in (("h", "haul: h"), ("v", "haul: v"), ("idler", "haul: idler")):
        box(x, 780, 120, 70, [cap], fr("env_dozer_haul", tag), k=2)
        x += 130
    d.text((410, 784), "haul line (DZ2-AD6): pulley stub -> h tiles along the ceiling -> idler over the brace -> v tiles down to the horn",
           fill=GREY, font=f)
    d.text((410, 800), "brace: intact -> weakened (1 hit) -> snapped on the release (swings snapped / snapped_swing every 8 ticks "
           "for 48 ticks)", fill=GREY, font=f)
    x = 10
    for tag in ("intact", "weakened", "snapped", "snapped_swing"):
        im = fr("prop_stop_frame_brace", tag)
        box(x, 866, 120, 140, [tag], im, k=1)
        x += 130
    x = 560
    for tag in ("rider_a_tumble_0", "rider_a_tumble_1", "rider_a_tumble_2", "engineer_slump_0"):
        stem = "dozer_engineer_slump_h120" if tag.startswith("engineer") else "dozer_rider_tumble_h120"
        im = fr(stem, tag)
        box(x, 866, 220, 140, [tag], im.crop(im.getchannel("A").getbbox()), k=2)
        x += 230
    return img


CYAN_T = (110, 220, 230)


def quantise(frames: list[Image.Image], colors: int = 128) -> list[Image.Image]:
    """One shared palette (128 colours by default) for the whole animation (deterministic, no dither)."""
    w, h = frames[0].size
    strip = Image.new("RGB", (w, h * min(len(frames), 12)))
    step = max(1, len(frames) // 12)
    for n, i in enumerate(range(0, len(frames), step)):
        if n >= 12:
            break
        strip.paste(frames[i], (0, h * n))
    pal = strip.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    return [f.quantize(palette=pal, dither=Image.Dither.NONE) for f in frames]


def save_gif(frames: list[Image.Image], path: Path, colors: int = 128) -> None:
    q = quantise([f.convert("RGB") for f in frames], colors)
    q[0].save(path, save_all=True, append_images=q[1:], duration=20, loop=0, optimize=False, disposal=1)


def glance_frames(k: int) -> list[Image.Image]:
    """The rear glance tick by tick (the strike at tick 0): the machine chasing on lit Floor 1, the frame falling
    behind it, the beam striking the rear deck, tipping off behind the exhaust and the remnant dropping in; the
    machine keeps chasing with its live parts. Composed from the atlases with the presenter's placement rules
    (``src/dozer_art.rs``: GLANCE_*, the haul line) and a chase at 160 px/s; the hull's rear is at the footprint's
    x + 60 on the strike tick."""
    floor = gr.Floor()
    vx = floor.view[0]
    speed = 160 / 60
    foot_x = vx + 150                                  # the footprint's left edge (80 wide), room 3's layout
    strike_o = foot_x + 60 + 84                        # O when the hull's rear (O - 84) is 60 px into the footprint
    brace_x = foot_x + 160                             # the brace is 160 px right of the footprint (x 800 vs 640)
    hull_img = {t: frame_of(f"{DOZER}/dozer_hull_h120.grid.json", t)[0] for t in ("body",)}
    pulley, rp = frame_of(f"{PROPS}/prop_stop_frame.grid.json", "pulley")
    frame_img = {t: frame_of(f"{PROPS}/prop_stop_frame.grid.json", t)[0] for t in ("raised", "warn_0", "warn_1", "falling")}
    gl = {t: frame_of(f"{PROPS}/prop_stop_frame_glance.grid.json", t)[0] for t in ("glance_0", "glance_1", "glance_2", "shadow")}
    shadows = [frame_of(f"{PROPS}/prop_stop_frame_shadow.grid.json", f"s{i}")[0] for i in range(4)]
    rem, remp = frame_of(f"{PROPS}/prop_stop_frame_remnant.grid.json", "landed")
    brace_i, bp = frame_of(f"{PROPS}/prop_stop_frame_brace.grid.json", "intact")
    brace_s, _ = frame_of(f"{PROPS}/prop_stop_frame_brace.grid.json", "snapped")
    carl, cp = frame_of(CARL, "idle")
    rig = json.loads((ROOT / DOZER / "dozer_rig_h120.json").read_text())
    anchors = {p["part"]: tuple(p["anchor"]) for p in rig["parts"] if isinstance(p.get("anchor"), list)}
    steam = [smoke(f"steam_{i}") for i in range(3)]
    out = []
    ticks = range(-46, 42)
    for t in ticks:
        o = strike_o + speed * t
        o_x = round(o)
        dist = max(0.0, o - strike_o + 0.0)
        # the live chase machine: tread by distance, wheel phase, the hull and riders rattle
        belt = int(o) % 9
        wheel = int((o / (44 * math.pi / 24))) % 4
        up1 = 1 if t % 6 < 3 else 0
        parts = []

        def add(z, img_piv, anchor, dy=0):
            parts.append((z, img_piv[0], img_piv[1], (anchor[0], anchor[1] + dy)))
        add(1, frame_of(f"{DOZER}/dozer_hull_h120.grid.json", "body"), anchors["hull_body"], up1)
        add(2, frame_of(f"{DOZER}/dozer_tread_h120.grid.json", f"belt_{belt}"), anchors["tread"])
        for name, tag in (("engineer", "engineer_hold"), ("rider_a", "rider_a_brandish"), ("rider_b", "rider_b_hold")):
            add(4 if name == "rider_b" else 3, rider(tag), anchors[name], up1)
        add(5, frame_of(f"{DOZER}/dozer_front_cab_h120.grid.json", "front_cab"), anchors["hull_front_cab"], up1)
        add(5, frame_of(f"{DOZER}/dozer_front_tub_h120.grid.json", "front_tub"), anchors["hull_front_tub"], up1)
        add(6, frame_of(f"{DOZER}/dozer_wheel_h120.grid.json", f"phase_{wheel}"), anchors["wheel"])
        add(7, frame_of(f"{DOZER}/dozer_yoke_h120.grid.json", "static"), anchors["yoke"])
        W, H = 380, 184
        ox, fy = 96, H - 1
        machine = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        for _z, img, piv, (ax, ay) in sorted(parts, key=lambda q: q[0]):
            machine.alpha_composite(img, (ox + ax - piv[0], fy - ay - piv[1]))
        entries = [(brace_s if t >= -38 else brace_i, bp, brace_x, 0)]
        entries += haul_entries(foot_x, brace_x)
        entries.append((pulley, rp, foot_x, FRAME_LIFT))
        # the stop frame: raised until the release (-38), the warning (30 ticks), the fall (8), the glance (0..13)
        if t < -38:
            entries.append((frame_img["raised"], (8, 59), foot_x, FRAME_LIFT))
        elif t < -8:
            age = t + 38
            entries.append((shadows[min(3, age * 4 // 30)], (0, 3), foot_x, 0))
            entries.append((frame_img["warn_0" if (t // 3) % 2 == 0 else "warn_1"], (8, 59), foot_x, FRAME_LIFT))
        elif t < 0:
            n = t + 8 + 1                                # falling ticks done after this one, 1..8
            drop = FRAME_LIFT * n * n // 64
            entries.append((shadows[3], (0, 3), foot_x, 0))
            entries.append((frame_img["falling"], (8, 59), foot_x, FRAME_LIFT - drop))
        entries.append((machine, (ox, fy), o_x, 0))
        if t >= 0:
            hull_left = o_x - 84
            deck_y = 108                                      # the rear deck (the cab roof) above the walking line
            idx = 0 if t < 4 else (1 if t < 8 else 2)
            if t < 14:
                dx, dy = a2.GLANCE_OFFSETS[idx]
                sx = hull_left + 8 + 40
                # The compositor draws a contact shadow under each sprite box: the pivot is moved to the canvas
                # bottom row (87) so that shadow falls on the walking line, and the lift follows (-43).
                if idx < 2:
                    entries.append((gl["shadow"], (44, 87), sx, deck_y))
                entries.append((gl[f"glance_{idx}"], (44, 87), sx + dx, deck_y + 9 + dy - 43))
            else:
                entries.append((rem, remp, foot_x, 0))
                age = t - 14
                for xx in (foot_x + 2, foot_x + 78):
                    for n in range(2):
                        a = age - n * 6
                        if 0 <= a < 9:
                            puff, pp = steam[min(2, a // 3)]
                            entries.append((puff, pp, xx, 6 + 0))
        entries.append((carl, cp, brace_x + 250, 0))   # Carl, late with the release, far ahead of the machine
        out.append(floor.render(entries, 1))
    return out


def reversal_gif(frames_dir: Path, path: Path, crop: tuple[int, int, int, int] | None, scale: int,
                 colors: int) -> None:
    """The Bevy reversal captures (one PNG per tick) as a 50 fps GIF (2 cs per tick: 0.83x real time)."""
    pngs = sorted(frames_dir.glob("frame_*.png"))
    if not pngs:
        raise SystemExit(f"no frames in {frames_dir}")
    with tempfile.TemporaryDirectory() as tmp:
        for n, f in enumerate(pngs):
            im = Image.open(f).convert("RGB")
            if crop:
                im = im.crop(crop)
            if scale != 1:
                im = im.resize((im.width * scale, im.height * scale), Image.Resampling.NEAREST)
            im.save(Path(tmp) / f"f{n:04d}.png")
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-framerate", "50", "-i", str(Path(tmp) / "f%04d.png"),
                        "-vf", f"split[a][b];[a]palettegen=max_colors={colors}:stats_mode=diff[p];[b][p]paletteuse=dither=none",
                        "-loop", "0", str(path)], check=True)


BEVY_STILLS = ("gate_closed", "gate_shake", "brace_weakened", "warning", "fall", "impact_pitch0", "impact_pitch1",
               "impact_tumble", "impact_lying", "disabled", "gate_burst", "crew_fight")
BEVY_CROPS = ("brace_weakened", "warning", "fall", "impact_pitch0", "impact_pitch1", "impact_tumble", "impact_lying",
              "disabled")


def bevy_sheets(out: Path) -> dict[str, Image.Image]:
    """The contact sheet (every 640 x 360 still at half size, labelled) and the enlarged crops (x2 of the machine and
    brace region) of the Bevy stills written by ``make_dozer_doz02_evidence.sh``; empty when the stills are missing."""
    paths = {n: out / f"doz02_bevy_{n}_640x360.png" for n in BEVY_STILLS}
    if not all(p.is_file() for p in paths.values()):
        return {}
    cell_w, cell_h = 320, 180
    cols = 3
    rows = (len(BEVY_STILLS) + cols - 1) // cols
    contact_img = Image.new("RGB", (cols * (cell_w + 6) + 6, rows * (cell_h + 18) + 6), BG)
    d = ImageDraw.Draw(contact_img)
    for n, name in enumerate(BEVY_STILLS):
        im = Image.open(paths[name]).convert("RGB").resize((cell_w, cell_h), Image.Resampling.NEAREST)
        x, y = 6 + (n % cols) * (cell_w + 6), 6 + (n // cols) * (cell_h + 18)
        contact_img.paste(im, (x, y + 14))
        d.text((x, y), name, fill=YELLOW, font=gr.font())
    box = (0, 100, 330, 300)
    cw, ch = (box[2] - box[0]) * 2, (box[3] - box[1]) * 2
    crops = Image.new("RGB", (2 * (cw + 6) + 6, 4 * (ch + 18) + 6), BG)
    d = ImageDraw.Draw(crops)
    for n, name in enumerate(BEVY_CROPS):
        im = Image.open(out / f"doz02_bevy_{name}_640x360.png").convert("RGB").crop(box)
        im = im.resize((cw, ch), Image.Resampling.NEAREST)
        x, y = 6 + (n % 2) * (cw + 6), 6 + (n // 2) * (ch + 18)
        crops.paste(im, (x, y + 14))
        d.text((x, y), name, fill=YELLOW, font=gr.font())
    return {"doz02_bevy_contact.png": contact_img, "doz02_bevy_crops_x2.png": crops}


def write_sums(out: Path) -> None:
    lines = []
    for f in sorted(out.iterdir()):
        if f.is_file() and f.name not in ("SHA256SUMS", "README.md"):
            lines.append(f"{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}")
    (out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--reversal-frames", type=Path, default=None,
                        help="the Bevy reversal captures (one PNG per tick) of make_dozer_doz02_evidence.sh")
    args = parser.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    outs = {
        "doz02_contact_x1.png": contact(1),
        "doz02_contact_x4.png": contact(4),
        "doz02_frames_x6.png": frames_x6(),
        "doz02_key_sheet_x4.png": key_sheet(),
        "doz02_state_diagram.png": state_diagram(),
    }
    before, after = lit("intact"), lit("disabled")
    outs["doz02_lit_before_640x360.png"] = before
    outs["doz02_lit_after_640x360.png"] = after
    outs["doz02_lit_pair_x2.png"] = stack([up(before.convert("RGBA"), 2), up(after.convert("RGBA"), 2)])
    for name, img in {**outs, **bevy_sheets(args.out)}.items():
        img.convert("RGB").save(args.out / name, optimize=False)
        print(f"wrote {name}")
    g1 = glance_frames(1)
    crop = (130, 100, 400, 290)
    save_gif(g1, args.out / "doz02_glance_x1.gif")
    save_gif([im.crop(crop).resize(((crop[2] - crop[0]) * 3, (crop[3] - crop[1]) * 3), Image.Resampling.NEAREST)
              for im in g1], args.out / "doz02_glance_x3.gif", 64)
    print(f"wrote doz02_glance_x1.gif and doz02_glance_x3.gif ({len(g1)} frames)")
    if args.reversal_frames is not None:
        # Cropped (the HUD and the empty right side are cut) and palette-limited to stay under the 3.5 MiB GIF budget.
        reversal_gif(args.reversal_frames, args.out / "doz02_reversal_x1.gif", (0, 70, 420, 310), 1, 48)
        reversal_gif(args.reversal_frames, args.out / "doz02_reversal_x3.gif", (30, 140, 260, 262), 3, 32)
        print("wrote doz02_reversal_x1.gif and doz02_reversal_x3.gif")
    write_sums(args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
