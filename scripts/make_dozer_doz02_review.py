#!/usr/bin/env python3
"""Review renders for the DOZ-02 design keys (Dozer damage states and the stop frame).

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases):

    python3 scripts/make_dozer_doz02_review.py [--out DIR]

Reads the DOZ-01 and DOZ-02 pixel-grid sources and the two rig contracts (``dozer_rig_h120.json``,
``dozer_damage_h120.json``) and writes deterministically into DIR (default docs/evidence/doz-02):

- ``doz02_contact_x1.png`` / ``doz02_contact_x4.png``: every DOZ-02 frame at native size and at x4;
- ``doz02_key_sheet_x4.png``: the design keys at x4 beside their DOZ-01 originals (the gate pack's
  enlarged key sheet): live wheel vs jammed wheel, intact vs disabled machine, the impact pitch key,
  the hanging frame, the remnant and the brace;
- ``doz02_lit_before_640x360.png`` / ``doz02_lit_after_640x360.png``: the lit Floor 1 corridor at
  gameplay scale (A1 compositor): the DOZ-01 machine under the raised frame with Carl at the brace,
  then the disabled machine with the jammed wheel and the landed remnant;
- ``doz02_lit_pair_x2.png``: the two lit compositions stacked at x2.

Composed PNGs, not Bevy captures. Presentation-only art preview; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
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
MACHINE_O_X = 300          # rig origin O, layout px from the view's left edge
FRAME_X = 380              # stop-frame mass rectangle left edge, view x (over the tub and the wheel)
FRAME_LIFT = 168
BRACE_X = 520              # brace target left edge, view x
CARL_X = 512               # Carl in front of the brace


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
    level, riders gone, the driver alone (DOZ-01 hold pose as the stand-in until the slump key exists)."""
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
        tags = {"engineer": "engineer_hold"}
    else:
        tags = {"engineer": "engineer_lurch_forward", "rider_a": "rider_a_lurch_forward",
                "rider_b": "rider_b_lurch_forward"}
    for name, tag in tags.items():
        add(4 if name == "rider_b" else 3, rider(tag), anchors[name], seat_dy[name])
    for name in ("front_cab", "front_tub"):
        if dmg:
            add(5, frame_of(f"{DOZER}/dozer_{name}_damage_h120.grid.json", state), anchors[f"hull_{name}"])
        else:
            add(5, frame_of(f"{DOZER}/dozer_{name}_h120.grid.json", name), anchors[f"hull_{name}"])
    add(6, frame_of(f"{DOZER}/dozer_wheel_jammed_h120.grid.json", "jammed") if dmg
        else frame_of(f"{DOZER}/dozer_wheel_h120.grid.json", "phase_0"), anchors["wheel"])
    add(7, frame_of(f"{DOZER}/dozer_yoke_damage_h120.grid.json", "bent") if dmg
        else frame_of(f"{DOZER}/dozer_yoke_h120.grid.json", "static"), anchors["yoke"])
    W, H = 300, 184
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


def haul_line(start_x: int, chain_y: int, brace_cx: int, brace_top: int, ground: int) -> tuple[Image.Image, tuple[int, int], int, int]:
    """The haul chain from the pulley block's stub along the ceiling to an idler over the brace and down to the
    brace's horn (drawn with the authoring script's chain pattern; the execution step tiles it for the room)."""
    w = brace_cx - start_x + 4
    h = brace_top - chain_y + 2
    rows = [["."] * w for _ in range(h)]
    a2.chain_h(rows, 1, 0, w - 4, phase=0)
    a2.chain_v(rows, w - 3, 3, h - 1, phase=1)
    rs = ad.Raster(7, 7, 3, 6, 1.0)                     # the idler pulley at the corner
    rs.disc(0.5, 3.5, 2.9, ad.BRASS, outline=True)
    rs.render()
    rs.put(0.5, 3.5, "L")
    ad.overlay(rows, rs.key, w - 7, -2)
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    img.paste(gr.up(rows_img(rows), 1), (0, 0))
    return img, (0, h - 1), start_x, ground - brace_top + 1


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
    brace, bp = frame_of(f"{PROPS}/prop_stop_frame_brace.grid.json", "intact")
    ground = floor.ground(vx + BRACE_X)
    frame_top = ground - FRAME_LIFT - rp[1]
    chain = haul_line(vx + FRAME_X - rp[0] + a2.FRAME_CANVAS[0], frame_top + 4 - 1, vx + BRACE_X + 8,
                      ground - bp[1], ground)
    entries = [(brace, bp, vx + BRACE_X, 0), (chain[0], chain[1], chain[2], chain[3]),
               (pulley, rp, vx + FRAME_X, FRAME_LIFT)]
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
    carl, cp = frame_of(CARL, "idle")
    entries.append((carl, cp, vx + CARL_X, 0))
    return floor.render(entries, 1)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    outs = {
        "doz02_contact_x1.png": contact(1),
        "doz02_contact_x4.png": contact(4),
        "doz02_key_sheet_x4.png": key_sheet(),
    }
    before, after = lit("intact"), lit("disabled")
    outs["doz02_lit_before_640x360.png"] = before
    outs["doz02_lit_after_640x360.png"] = after
    outs["doz02_lit_pair_x2.png"] = stack([up(before.convert("RGBA"), 2), up(after.convert("RGBA"), 2)])
    for name, img in outs.items():
        img.convert("RGB").save(args.out / name, optimize=False)
        print(f"wrote {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
