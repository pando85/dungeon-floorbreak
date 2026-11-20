#!/usr/bin/env python3
"""Evidence for ART-CARL-CF02 (Carl combat-acting sheet): lit Floor 1 compositions, in-scene 1x GIFs, timing diagram.

Everything is composed from the grid sources and the manifest tick contract (nothing is a Bevy capture, nothing is
wired into a runtime). The toolkit part (contact sheets, per-tag GIFs) is ``python3 -m scripts.pixelart evidence``;
this script adds what the toolkit has no verb for. Needs the Floor 1 kit atlases: run ``bash scripts/export_assets.sh``
once in a fresh checkout. Output is deterministic (no timestamps).

    python3 scripts/make_carl_cf02_evidence.py

The in-scene GIFs are tick-driven mock-ups for the art review (a parabola for the jump, a straight plunge, constant
retreat speed); they are NOT the gameplay physics of COMBAT-FUN-02 (the Bevy wiring is CF02-I). The lying goblin is the
approved GOB-01 ``down`` frame, a stand-in until the ART-GOB-CF02 stomped frames exist.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from scripts.pixelart.common import (BG, CARL_X, DONUT_X, GREY, WHITE, YELLOW, cell, flip, gif_delays,  # noqa: E402
                                     grid_images, label, save_gif)
from scripts.pixelart.scene import Floor  # noqa: E402

EVIDENCE = ROOT / "docs/evidence/carl-cf02"
GRID = ROOT / "assets/source/characters/carl/carl_h52_cf02.grid.json"
MANIFEST = ROOT / "assets/manifests/carl_book1_h52_cf02.json"
MELEE = ROOT / "assets/source/characters/carl/carl_h52_melee.grid.json"
LOCO = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
GOB = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
DONUT = ROOT / "assets/source/characters/donut/donut_h40.grid.json"

ORDER = ["stomp_knee_up", "stomp_stamp", "stomp_recover", "air_kick_chamber", "air_kick_boot", "dive_tuck",
         "dive_plunge", "dive_heavy_landing", "bailout_pushoff", "bailout_scramble", "flinch",
         "frustration_shake", "frustration_flap"]
PIV = (25, 55)
GPIV = (28, 55)
STEP = 2            # ticks per GIF frame


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


class Art:
    def __init__(self) -> None:
        self.cf = dict(zip(ORDER, grid_images(load(GRID))))
        loco, melee = grid_images(load(LOCO)), grid_images(load(MELEE))
        self.idle = loco[0]
        self.jump_start, self.rise, self.fall, self.land = loco[11], loco[12], loco[13], loco[14]
        self.kick = melee[8:15]
        self.gob = grid_images(load(GOB))
        self.donut = grid_images(load(DONUT))[0]


def sprite(img, x, lift=0, shadow=True, left=False):
    if left:
        im, piv = flip(img, GPIV)
        return (im, piv, x, lift, shadow)
    return (img, PIV, x, lift, shadow)


def goblin(art, frame, x, lift=0):
    img, piv = flip(art.gob[frame], GPIV)
    return (img, piv, x, lift, True)


def lying_goblin(art, centre_x):
    """GOB-01 ``down`` frame 22, mirrored, with its opaque bbox centred on ``centre_x``."""
    img, piv = flip(art.gob[22], GPIV)
    box = img.getbbox()
    return (img, piv, int(centre_x - ((box[0] + box[2]) / 2 - piv[0])), 0, True)


# ------------------------------------------------------------------ tick-driven scenes

def parabola(t, total, apex):
    u = t / total
    return max(0.0, 4 * apex * u * (1 - u))


def scene_stomp(art, x0):
    """Q02: knee-up t0-8 (root step 6 px over t1-6), stamp t9-15 with the 6-tick hit-stop on the stamp, recover."""
    frames = []
    for t in range(0, 6):
        frames.append((art.idle, x0, 0))
    for t in range(9):
        frames.append((art.cf["stomp_knee_up"], x0 + round(6 * min(t, 6) / 6), 0))
    for t in range(7 + 6):
        frames.append((art.cf["stomp_stamp"], x0 + 6, 0))
    for t in range(10):
        frames.append((art.cf["stomp_recover"], x0 + 6, 0))
    for t in range(10):
        frames.append((art.idle, x0 + 6, 0))
    goblin_e = lying_goblin(art, x0 + 6 + 15)
    return [([goblin_e, sprite(img, x, lift)]) for img, x, lift in frames]


def scene_air_kick(art, x0):
    """Q50: jump, chamber t0-4, boot t5-14 (hit-stop 5 held on the boot), fall, 4 ticks of land."""
    out, T = [], 36
    gx = x0 + 46
    for t in range(4):
        out.append((art.idle, x0, 0, 0))
    for t in range(4):
        out.append((art.jump_start, x0, 0, 0))
    air = []
    for t in range(T):
        if t < 8:
            img = art.rise
        elif t < 13:
            img = art.cf["air_kick_chamber"]
        elif t < 23:
            img = art.cf["air_kick_boot"]
        else:
            img = art.fall
        air.append(img)
    hold = [t for t in range(T) if 13 <= t < 18]        # hit-stop: the sim pauses on the first boot ticks
    t_sim, xf = 0, float(x0)
    for t, img in enumerate(air):
        xf += 0.9 if t < 13 else -0.45         # forward until the boot, a small recoil after the hit
        out.append((img, round(xf), round(parabola(t_sim, T, 26)), 0))
        if t in hold[1:] and t != hold[-1]:
            continue
        t_sim += 1
    xl = out[-1][1]
    for t in range(4):
        out.append((art.land, xl, 0, 0))
    for t in range(10):
        out.append((art.idle, xl, 0, 0))
    scene = []
    for i, (img, x, lift, _) in enumerate(out):
        gob = art.gob[(i // 10) % 4]
        scene.append([goblin_from(gob, gx), sprite(img, x, lift)])
    return scene


def goblin_from(img, x):
    im, piv = flip(img, GPIV)
    return (im, piv, x, 0, True)


def scene_dive(art, x0):
    """Q52/Q56 whiff: hop, tuck 6 ticks, plunge to the floor, heavy landing 10 ticks, idle."""
    out = []
    for t in range(4):
        out.append((art.idle, x0, 0))
    for t in range(4):
        out.append((art.jump_start, x0, 0))
    x = x0
    for t in range(14):
        x += 1.0
        lift = 8 + 22 * min(1.0, t / 8)
        out.append((art.rise if t < 6 else art.fall, round(x), round(lift)))
    for t in range(6):
        x += 0.5
        out.append((art.cf["dive_tuck"], round(x), 30))
    lift = 30.0
    while lift > 0:
        lift = max(0.0, lift - 2.0)
        x += 1.4
        out.append((art.cf["dive_plunge"], round(x), round(lift)))
    for t in range(10):
        out.append((art.cf["dive_heavy_landing"], round(x), 0))
    for t in range(12):
        out.append((art.idle, round(x), 0))
    return [[goblin_from(art.gob[(i // 10) % 4], x0 + 95), sprite(img, xx, lift)] for i, (img, xx, lift) in enumerate(out)]


def scene_bailout(art, x0):
    """Q54: a missed push kick, a goblin swing coming, Carl bails out (pushoff t0-2, scramble ~30 ticks), land 4."""
    out = []
    kick = [(art.kick[0], 4), (art.kick[1], 7), (art.kick[2], 2), (art.kick[3], 6)]
    for img, n in kick:
        out += [(img, x0, 0)] * n
    gx = x0 + 40
    xs = x0
    for t in range(3):
        out.append((art.cf["bailout_pushoff"], xs, 0))
    air = 30
    for t in range(air):
        xs -= 1.3
        out.append((art.cf["bailout_scramble"], round(xs), round(parabola(t, air, 13))))
    for t in range(4):
        out.append((art.land, round(xs), 0))
    for t in range(14):
        out.append((art.idle, round(xs), 0))
    # goblin: attack_tell frames 10 (6t) and 11 (18t) from t0, swing 12 (2t) 13 (4t), recover 14 (12t) 15 (8t), idle
    seq = [10] * 6 + [11] * 18 + [12] * 2 + [13] * 4 + [14] * 12 + [15] * 8
    gob = [seq[i] if i < len(seq) else (i // 10) % 4 for i in range(len(out))]
    return [[goblin(art, gob[i], gx), sprite(img, x, lift)] for i, (img, x, lift) in enumerate(out)]


def strip(floor, art, name, builder, x0, ticks_at):
    """Debug/review still: the scene at the given ticks side by side (1x)."""
    per = builder(art, x0)
    vx, vy = floor.view[0], floor.view[1]
    gy = floor.ground(x0) - vy
    left = x0 - 90 - vx
    box = (max(0, left), max(0, gy - 90), left + 250, gy + 16)
    ims = [floor.render(per[min(t, len(per) - 1)], 1).crop(box) for t in ticks_at]
    out = Image.new("RGB", (ims[0].width * len(ims), ims[0].height))
    for i, im in enumerate(ims):
        out.paste(im, (i * im.width, 0))
    return out


def scene_gif(floor, art, name, builder, x0):
    entries_per_tick = builder(art, x0)
    vx, vy = floor.view[0], floor.view[1]
    gy = floor.ground(x0) - vy
    left = x0 - 90 - vx
    box = (max(0, left), max(0, gy - 90), left + 250, gy + 16)
    ticks = list(range(0, len(entries_per_tick), STEP))
    one = [floor.render(entries_per_tick[t], 1).crop(box) for t in ticks]
    delays = gif_delays([STEP] * len(one))
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    save_gif(one, delays, EVIDENCE / f"scene_{name}_x1.gif")
    save_gif([im.resize((im.width * 3, im.height * 3), Image.NEAREST) for im in one], delays,
             EVIDENCE / f"scene_{name}_x3.gif")
    return len(entries_per_tick)


# ------------------------------------------------------------------ stills

def lit_still(floor, entries, name, box=None):
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    one = floor.render(entries, 1)
    if box:
        vx, vy = floor.view[0], floor.view[1]
        one = one.crop((box[0] - vx, box[1] - vy, box[2] - vx, box[3] - vy))
    one.save(EVIDENCE / f"{name}_x1.png")
    one.resize((one.width * 3, one.height * 3), Image.NEAREST).save(EVIDENCE / f"{name}_x3.png")


def sets(floor, art):
    """Idle Carl, Donut and a goblin for scale, then the 13 frames in atlas order (two scenes, 7 and 6 frames)."""
    base = [(art.donut, (46, 43), DONUT_X, 0), (art.idle, PIV, CARL_X, 0)]
    for part, idx, x0 in (("a", range(0, 7), 520), ("b", range(7, 13), 520)):
        entries = list(base)
        for n, i in enumerate(idx):
            entries.append((art.cf[ORDER[i]], PIV, x0 + 56 * n, 0))
        entries.append(goblin(art, 0, x0 + 56 * len(idx) + 8))
        gy = floor.ground(CARL_X) - floor.view[1]
        out = floor.render(entries, 1)
        out.save(EVIDENCE / f"carl_set_{part}_floor1_640x360.png")
        floor.render(entries, 3).save(EVIDENCE / f"carl_set_{part}_floor1_1920x1080.png")


def reads(floor, art):
    """The two 1x checks of the acceptance targets: chamber against tuck, frustration pair against idle."""
    x0 = 480
    ent = [sprite(art.idle, x0), sprite(art.cf["air_kick_chamber"], x0 + 56, 0), sprite(art.cf["dive_tuck"], x0 + 112, 0),
           sprite(art.cf["air_kick_boot"], x0 + 168, 0), sprite(art.cf["dive_plunge"], x0 + 224, 0)]
    gy = floor.ground(x0) - floor.view[1]
    box = (x0 - 36, floor.ground(x0) - 70, x0 + 224 + 36, floor.ground(x0) + 10)
    lit_still(floor, ent, "check_chamber_vs_tuck", box)
    ent = [sprite(art.idle, x0), sprite(art.cf["frustration_shake"], x0 + 56), sprite(art.cf["frustration_flap"], x0 + 112),
           sprite(art.cf["flinch"], x0 + 168), sprite(art.idle, x0 + 224), goblin(art, 0, x0 + 284)]
    box = (x0 - 36, floor.ground(x0) - 70, x0 + 320, floor.ground(x0) + 10)
    lit_still(floor, ent, "check_frustration_pair", box)


# ------------------------------------------------------------------ timing diagram

def timing_diagram(art):
    man = load(MANIFEST)["cf02_acting"]
    PHASE = {"startup": (90, 140, 255), "active": (255, 150, 40), "recovery": (120, 200, 120), "reaction": (200, 120, 200)}
    K, TICK_PX, PAD = 2, 5, 10
    names = {n: i for i, n in enumerate(ORDER)}
    rows = []
    for mname, mv in man["moves"].items():
        rows.append((mname, mv))
    width = 1180
    out = Image.new("RGBA", (width, 40 + len(rows) * (56 * K + 70)), BG + (255,))
    d = ImageDraw.Draw(out)
    label(d, (PAD, 4), "ART-CARL-CF02 frame layout, pivot and 60 Hz timing (Carl 48 x 56, pivot (25, 55)). Frames face right.", WHITE, 12)
    label(d, (PAD, 20), "cyan = contact row 55, yellow box = pivot; airborne frames sit 2 px higher (bailout_scramble 1 px). Bar length = ticks x 5 px; "
          "existing frames are named in grey.", WHITE, 12)
    y = 40
    for mname, mv in rows:
        label(d, (PAD, y), f"{mname}: {mv['total_ticks']} ticks - {mv['role']}", YELLOW, 12)
        y += 18
        x = PAD
        for fr in mv["frames"]:
            ticks = fr["ticks"][1] - fr["ticks"][0] + 1
            nm = fr["name"]
            if nm in names:
                c = cell(art.cf[nm], K, PIV, 55).convert("RGBA")
                out.alpha_composite(c, (x, y))
            else:
                d.rectangle([x, y, x + 48 * K - 1, y + 56 * K - 1], outline=GREY)
            label(d, (x + 2, y + 56 * K + 2), nm + (" KEY" if fr.get("key") else ""), YELLOW if fr.get("key") else GREY, 11)
            label(d, (x + 2, y + 56 * K + 15), f"t{fr['ticks'][0]}-{fr['ticks'][1]}  {ticks}t", GREY, 11)
            d.rectangle([x, y + 56 * K + 32, x + ticks * TICK_PX - 1, y + 56 * K + 42], fill=PHASE[fr["phase"]])
            x += max(48 * K, ticks * TICK_PX) + 14
        y += 56 * K + 52
    label(d, (PAD, out.height - 20), "phase colours: blue startup, orange active, green recovery, violet reaction (idle only).", GREY, 11)
    out.convert("RGB").crop((0, 0, width, y + 22)).save(EVIDENCE / "timing_diagram.png")


def main() -> int:
    art = Art()
    floor = Floor()
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    sets(floor, art)
    reads(floor, art)
    timing_diagram(art)
    for name, builder, x0 in (("stomp_lying_goblin", scene_stomp, 560), ("air_kick_jump_in", scene_air_kick, 540),
                              ("dive_whiff_landing", scene_dive, 500), ("bailout_missed_kick", scene_bailout, 600)):
        n = scene_gif(floor, art, name, builder, x0)
        print(f"{name}: {n} ticks")
    print(f"wrote evidence in {EVIDENCE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
