#!/usr/bin/env python3
"""CHOP-01 authoring record: Goblin Copper Chopper palette, rule-based key poses and manifest.

Usage:

    python3 scripts/author_chop01.py keys [--check] [--preview PNG]
    python3 scripts/author_chop01.py scale [--out PNG]
    python3 scripts/author_chop01.py sheet

``keys`` (re)writes ``assets/source/characters/goblin/chopper/chop_01.grid.json`` (frames 0, 3, 8,
10, 11, 15 are the keys, see ``docs/02-art/chop-01-spec.md`` section 7) and
``assets/manifests/chop-01.json`` with the 60 Hz tick table. The in-between slots are copies of
the nearest key for now (``COPIES``). Every key is rasterized from one parametric rig (polygons,
capsules and discs at pixel centres; top-lit 3-tone ramps; a 1 px plum outline around each part)
plus literal stamps. ``--check`` compares the grid and manifest instead of writing.

``scale`` exports ``docs/evidence/chop-01/key0-scale.png``: key 0 at 1x on the lit Floor 1 floor
next to Carl and a GOB-01 goblin, plus a 4x crop.

No concept pixels are traced or sampled; colours are chosen by eye. Presentation-only art tooling;
nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHOP_DIR = ROOT / "assets/source/characters/goblin/chopper"
GRID = CHOP_DIR / "chop_01.grid.json"
MANIFEST = ROOT / "assets/manifests/chop-01.json"
SCALE_PNG = ROOT / "docs/evidence/chop-01/key0-scale.png"
KEYS_PNG = ROOT / "docs/evidence/chop-01/keys-x3.png"
CANVAS = (112, 64)
PIVOT = (56, 63)
TICK_HZ = 60

# chop-floor01-v0 (19 of 20). Skin, eye, tooth, scarf and rag keys are the GOB-01 values (same key
# letters) so the crew is the same species as the opening goblins. The machine is a warm copper ramp
# (c C D), a rust-red panel (R, p) and dark rubber (k); the cold iron/smoke pair (i, z) and the
# firebox/headlight pair (f, Y) are the only other families. Copper separates from the orange brick
# by value (darker, redder body, bright glow) and by the cold tyre.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("R", "red_mid", "#b8322eff", False),
    ("p", "red_dark", "#6e1a22ff", False),
    ("t", "rag_mid", "#4c3436ff", False),
    ("u", "rag_light", "#73524aff", False),
    ("c", "copper_dark", "#5a2e1cff", False),
    ("C", "copper_mid", "#b0602eff", False),
    ("D", "copper_light", "#e69a4eff", False),
    ("k", "rubber", "#2e2630ff", False),
    ("i", "iron", "#4a4450ff", False),
    ("z", "smoke", "#7a7286ff", False),
    ("f", "fire_orange", "#e2631cff", False),
    ("Y", "fire_hot", "#ffc84aff", True),
]
ACCENTS = {"e", "w", "Y"}
LAYERS = ["machine", "crew", "weapon"]

TAGS = [("idle", 0, 2), ("walk", 3, 6), ("attack_tell", 7, 8), ("attack_swing", 9, 10), ("hurt", 11, 12), ("down", 13, 15)]
TICKS = [12, 8, 12, 5, 5, 5, 5, 8, 22, 3, 9, 4, 8, 6, 10, None]
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once", "hurt": "once", "down": "hold"}
KEY_FRAMES = (0, 3, 8, 10, 11, 15)
COPIES = {1: 0, 2: 0, 4: 3, 5: 3, 6: 3, 7: 8, 9: 10, 12: 11, 13: 15, 14: 15}
INBETWEENS = tuple(sorted(COPIES))
GROUNDED_TAGS = [n for n, _a, _b in TAGS]


# ------------------------------------------------------------------ rasterizer

def disc(c, r):
    cx, cy = c
    return {(x, y) for y in range(int(cy - r) - 1, int(cy + r) + 2) for x in range(int(cx - r) - 1, int(cx + r) + 2)
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r + 0.25}


def capsule(a, b, r):
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    ln = dx * dx + dy * dy or 1.0
    out = set()
    for y in range(int(min(ay, by) - r) - 1, int(max(ay, by) + r) + 2):
        for x in range(int(min(ax, bx) - r) - 1, int(max(ax, bx) + r) + 2):
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / ln))
            if (x - ax - t * dx) ** 2 + (y - ay - t * dy) ** 2 <= r * r + 0.2:
                out.add((x, y))
    return out


def poly(pts):
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    out = set()
    for y in range(int(min(ys)), int(max(ys)) + 1):
        for x in range(int(min(xs)), int(max(xs)) + 1):
            inside = False
            j = len(pts) - 1
            for i in range(len(pts)):
                (xi, yi), (xj, yj) = pts[i], pts[j]
                if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
                    inside = not inside
                j = i
            if inside:
                out.add((x, y))
    return out


def chain(pts, r):
    out = set()
    for a, b in zip(pts, pts[1:]):
        out |= capsule(a, b, r)
    return out


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px: dict[tuple[int, int], tuple[str, str]] = {}

    def put(self, x, y, ch, layer):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[(x, y)] = (ch, layer)

    def paint(self, mask, layer, ramp, outline=True):
        """ramp = (dark, mid, light) or a single key. Top edge light, bottom edge dark."""
        if isinstance(ramp, str):
            ramp = (ramp, ramp, ramp)
        mask = {(x, y) for x, y in mask if 0 <= x < self.w and 0 <= y < self.h}
        for (x, y) in mask:
            up, down = (x, y - 1) in mask, (x, y + 1) in mask
            if not up and down:
                ch = ramp[2]
            elif up and not down:
                ch = ramp[0]
            else:
                ch = ramp[1]
            self.px[(x, y)] = (ch, layer)
        if outline:
            for (x, y) in mask:
                for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                    if n not in mask and 0 <= n[0] < self.w and 0 <= n[1] < self.h:
                        self.px[n] = ("K", layer)
        return mask

    def flat(self, mask, layer, ch):
        for (x, y) in mask:
            self.put(x, y, ch, layer)

    def despeckle(self):
        """Cluster rule: a non-accent pixel with no same-key 8-neighbour takes the most common key of its
        opaque 8-neighbours (ties broken by palette order). Repeats until stable, so V06 stays near zero."""
        order = {k: n for n, (k, *_r) in enumerate(PALETTE)}
        for _ in range(4):
            changed = False
            for (x, y), (ch, ly) in sorted(self.px.items()):
                if ch in ACCENTS:
                    continue
                around = [self.px[(x + dx, y + dy)][0] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and (x + dx, y + dy) in self.px]
                if not around or ch in around:
                    continue
                pick = max(set(around) - ACCENTS or set(around), key=lambda k: (around.count(k), -order[k]))
                self.px[(x, y)] = (pick, ly)
                changed = True
            if not changed:
                break

    def cels(self) -> dict[str, list[str]]:
        self.despeckle()
        out = {}
        for layer in LAYERS:
            rows = [["."] * self.w for _ in range(self.h)]
            for (x, y), (ch, ly) in self.px.items():
                if ly == layer:
                    rows[y][x] = ch
            out[layer] = ["".join(r) for r in rows]
        return out


# ------------------------------------------------------------------ rig

COPPER = ("c", "C", "D")
RUBBER = ("k", "k", "i")
SKIN = ("s", "m", "l")
RAG = ("t", "t", "u")
JACKET = ("u", "u", "C")  # leather lit from above and by the firebox; the belt carries the dark key
PANEL = ("p", "R", "R")
SKULL = ["wwwww", "wwwwwww", "wKKwKKw", "wwwKwww", ".wwwww.", ".wKwKw."]

DEFAULT = dict(
    pitch=0.0, pivot_x=26, dy=0, dx=0, lean=0, head=(0, 0), scarf=1.0, scarf_lift=0, glow=1, smoke=1, spoke=0.0,
    club=(100, 0), rider=(0, 0), rider_head=(0, 0), rider_arm=0, reach=0, shear=0.0, burst=None, puff_back=False, debris=False, mouth="snarl", eyes="e", sparks=(), wreck=False,
    driver_arm=0, fork=0,
)


class Rig:
    def __init__(self, pose):
        self.p = {**DEFAULT, **pose}
        self.cv = Canvas(*CANVAS)

    def P(self, x, y):
        p = self.p
        th = math.radians(p["pitch"])
        dxx, dyy = x - p["pivot_x"], y - 63
        xr = p["pivot_x"] + dxx * math.cos(th) + dyy * math.sin(th)
        yr = 63 - dxx * math.sin(th) + dyy * math.cos(th)
        xr += p["shear"] * (63 - yr)
        return (xr + p["dx"], yr + p["dy"])

    def Ps(self, pts):
        return [self.P(*q) for q in pts]

    # ---- machine pieces
    def bands(self, mask, xs, key="c"):
        """2 px dark clamp bands across a copper tube at rig x positions."""
        cols = {int(round(self.P(x, 50)[0])) + d for x in xs for d in (0, 1)}
        for (x, y) in mask:
            if x in cols:
                self.cv.px[(x, y)] = (key, "machine")

    def wheel(self, c, r, phase):
        cv = self.cv
        cx, cy = self.P(*c)
        if self.p["wreck"]:  # burst tyre, hub sunk: an ellipse resting on the contact row
            ry = r - 5
            cy = 63 - ry
            rub = {(x, y) for x in range(int(cx - r) - 1, int(cx + r) + 2) for y in range(int(cy - ry) - 1, 64)
                   if ((x - cx) / r) ** 2 + ((y - cy) / ry) ** 2 <= 1.02 or (y >= 62 and abs(x - cx) <= r + 1)}
            cv.paint(rub, "machine", ("k", "k", "i"))
            cv.flat({(x, y) for x in range(int(cx - r) + 4, int(cx + r) - 3) for y in range(int(cy - ry) + 2, int(cy) + 2)
                     if ((x - cx) / (r - 4)) ** 2 + ((y - cy + 1) / max(ry - 3, 1)) ** 2 <= 1.0}, "machine", "c")
            cv.flat(disc((cx, cy - 1), 1.2), "machine", "C")
            return
        cv.paint(disc((cx, cy), r), "machine", RUBBER)
        cv.flat(disc((cx, cy), r - 3), "machine", "C")
        cv.flat(disc((cx, cy), r - 4.2), "machine", "c")
        for n in range(3):
            a = phase + n * math.pi / 3
            for s in range(-(r - 4), r - 3):
                cv.put(cx + s * math.cos(a), cy + s * math.sin(a), "D", "machine")
        cv.flat(disc((cx, cy), 1.2), "machine", "D")
        cv.put(cx, cy, "K", "machine")

    def smoke(self, c, size, dark=False):
        cv = self.cv
        cx, cy = self.P(*c)
        if self.p["puff_back"]:
            size = 0
        blobs = {0: [(0, 0, 3.2), (-5, 1, 3), (-10, 2, 2.5), (-14, 2, 2)], 1: [(0, 0, 3)], 2: [(0, 0, 3.5), (-4, -2, 3), (3, -3, 2.5)], 3: [(0, 0, 4), (-5, -3, 3.5), (4, -4, 3), (-9, -4, 2.5)]}[size]
        m = set()
        for dx, dy, r in blobs:
            m |= disc((cx + dx, cy + dy), r)
        cv.paint(m, "machine", "i" if dark else ("i", "z", "z"), outline=False)

    def machine(self):
        p, cv, P = self.p, self.cv, self.P
        # rear wheel (mostly hidden by the tub) and chain
        self.wheel((58, 53), 10, p["spoke"] + 0.4)
        # frame rail, engine block, gear
        self.bands(cv.paint(chain(self.Ps([(58, 53), (78, 51), (90, 52)]), 1), "machine", "C"), (66, 84))
        cv.paint(poly(self.Ps([(64, 46), (82, 46), (83, 55), (64, 56)])), "machine", ("K", "i", "z"))
        gx, gy = P(74, 51)
        cv.paint(disc((gx, gy), 4), "machine", COPPER)
        cv.put(gx, gy, "K", "machine")
        cv.paint(chain(self.Ps([(74, 51), (62, 53)]), 0.5), "machine", "k", outline=False)
        # exhaust pipe low along the engine
        self.bands(cv.paint(chain(self.Ps([(84, 57), (56, 58)]), 1.2), "machine", COPPER), (63, 75))
        # boiler / firebox behind the driver, stack above it
        cv.paint(poly(self.Ps([(49, 34), (63, 34), (64, 47), (48, 47)])), "machine", COPPER)
        grill = poly(self.Ps([(52, 38), (60, 38), (60, 45), (52, 45)]))
        cv.flat(grill, "machine", "f" if p["glow"] >= 1 else "c")
        if p["glow"] >= 2:
            cv.flat(poly(self.Ps([(53, 39), (59, 39), (59, 44), (53, 44)])), "machine", "Y")
        elif p["glow"] == 1:
            cv.flat(poly(self.Ps([(54, 40), (58, 40), (58, 43), (54, 43)])), "machine", "Y")
        for gxp in (54, 57):  # grille bars
            for yy in range(38, 46):
                x, y = P(gxp, yy)
                cv.put(x, y, "c", "machine")
        sx0 = self.P(54, 34)
        stack_top = (20 if p["glow"] < 2 else 18) + (4 if p["wreck"] else 0)
        bend = -5 if p["wreck"] else 0  # the wreck's stack is knocked crooked, cap pointing back
        cv.paint(poly(self.Ps([(53, 33), (57, 33), (57 + bend, stack_top + 2), (53 + bend, stack_top + 2)])), "machine", COPPER)
        cv.paint(poly(self.Ps([(51 + bend, stack_top + 2), (59 + bend, stack_top + 2), (59 + bend, stack_top), (51 + bend, stack_top)])),
                 "machine", COPPER)
        _ = sx0
        if p["smoke"]:
            self.smoke((50 + bend, stack_top - 5), p["smoke"], dark=p["wreck"])
        # seat
        cv.paint(poly(self.Ps([(62, 40), (72, 40), (72, 43), (62, 43)])), "machine", ("K", "u", "u"))
        # fork, headlight, bars, front wheel
        fk = p["fork"]
        self.wheel((90, 52), 11, p["spoke"])
        cv.paint(chain(self.Ps([(78, 30), (90 + fk, 52)]), 1.2), "machine", "C")
        cv.paint(chain(self.Ps([(86, 41), (93, 38)]), 0.8), "machine", "c", outline=False)
        hx, hy = P(96, 38)
        cv.paint(disc((hx, hy), 4.2), "machine", COPPER)
        cv.flat(disc((hx + 0.5, hy + 0.5), 2.4), "machine", "Y" if p["glow"] >= 1 else ("i" if p["wreck"] else "f"))
        cv.put(hx + 0.5, hy - 2, "c", "machine")
        cv.put(hx + 0.5, hy + 3, "c", "machine")
        if p["glow"] >= 2:
            for dx, dy in ((5, -1), (5, 1), (6, 0), (4, -3), (4, 3)):
                cv.put(hx + dx, hy + dy, "f", "machine")
        cv.paint(chain(self.Ps([(78, 30), (77, 25), (83, 24)]), 0.8), "machine", "C")

    # ---- crew
    def head(self, c, layer, goggles, mouth, eyes, ear_dx=-5, r=4.5):
        cv = self.cv
        cx, cy = c
        cv.paint(disc((cx, cy), r) | poly([(cx + 4, cy - 1), (cx + 7, cy), (cx + 7, cy + 2), (cx + 4, cy + 2)]), layer, SKIN)
        cv.paint(poly([(cx - 3, cy - 2), (cx + ear_dx - 3, cy - 4 - (r > 5)), (cx - 2, cy + 1)]), layer, SKIN)
        ex, ey = cx + 2, cy - 1
        if goggles:
            cv.paint(poly([(cx - 5, cy - 4), (cx + 4, cy - 5), (cx + 5, cy - 2), (cx - 5, cy - 1)]), layer, ("c", "t", "t"))
            cv.flat(poly([(cx - 4, cy - 5), (cx + 2, cy - 6), (cx + 1, cy - 4), (cx - 4, cy - 4)]), layer, "u")
            cv.put(cx - 1, cy - 6, "D", layer)
            lens = "e" if eyes == "e" else "w"
            for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
                cv.put(ex + dx, ey + dy, lens, layer)
            cv.put(ex - 1, ey, "C", layer); cv.put(ex + 2, ey, "C", layer)
        else:
            cv.put(ex, ey, "e" if eyes == "e" else "w", layer)
            cv.put(ex + 1, ey, "e" if eyes == "e" else "w", layer)
            cv.put(ex - 1, ey - 1, "K", layer); cv.put(ex, ey - 1, "K", layer)
        my = cy + 2
        if mouth == "snarl":
            for dx in range(0, 5):
                cv.put(cx + 2 + dx, my, "K", layer)
            cv.put(cx + 3, my + 1, "w", layer); cv.put(cx + 5, my + 1, "w", layer)
        elif mouth == "open":
            for dx in range(0, 5):
                cv.put(cx + 2 + dx, my, "w" if dx % 2 == 0 else "K", layer)
                cv.put(cx + 2 + dx, my + 1, "p", layer)
            cv.put(cx + 3, my + 2, "R", layer); cv.put(cx + 4, my + 2, "R", layer)
            cv.put(cx + 5, my + 2, "K", layer); cv.put(cx + 2, my + 2, "K", layer)
        else:  # "O": surprised / hurt
            for dx, dy in ((3, 0), (4, 0), (2, 1), (5, 1), (3, 2), (4, 2)):
                cv.put(cx + dx, my + dy, "K", layer)
            cv.put(cx + 3, my + 1, "p", layer); cv.put(cx + 4, my + 1, "p", layer)

    def helmet(self, c, layer):
        cx, cy = c
        m = {(x, y) for (x, y) in disc((cx, cy - 1), 5.2) if y <= cy - 3 and x <= cx + 3}
        self.cv.paint(m, layer, ("t", "t", "u"))
        self.cv.put(cx - 1, cy - 6, "D", layer)

    def limb(self, a, b, r, layer, ramp):
        self.cv.paint(capsule(a, b, r), layer, ramp)

    def hand(self, c, layer):
        self.cv.flat(disc(c, 1.3), layer, "m")

    def driver(self):
        p, cv, P = self.p, self.cv, self.P
        lean = p["lean"]
        hip = P(66, 41)
        sh = P(70 + lean, 31 + (1 if lean >= 3 else 0))
        hd = P(73 + lean + p["head"][0], 24 + p["head"][1] + (3 if lean >= 3 else 0))
        # scarf streams back (left) from the neck
        s = p["scarf"]
        n0 = (sh[0] - 2, sh[1] - 2)
        lift = p["scarf_lift"]
        cv.paint(chain([n0, (n0[0] - 7 * s, n0[1] + 1 - lift), (n0[0] - 14 * s, n0[1] + 1 - lift * 2), (n0[0] - 20 * s, n0[1] + 4 - lift * 2)], 1.4), "crew", ("p", "R", "R"))
        # leg: thigh, shin, boot
        knee, foot = P(75, 45), P(77, 52)
        self.limb(hip, knee, 2.4, "crew", RAG)
        self.limb(knee, foot, 2.0, "crew", RAG)
        cv.paint(poly([(foot[0] - 2, foot[1] + 1), (foot[0] + 4, foot[1] + 1), (foot[0] + 4, foot[1] + 4), (foot[0] - 2, foot[1] + 4)]), "crew", ("K", "t", "u"))
        # torso, belt, harness
        torso = cv.paint(capsule(hip, sh, 3.6), "crew", JACKET)
        for (x, y) in torso:
            if (x - 1, y) not in torso and y < hip[1] - 2:  # firebox bounce on the back edge
                cv.px[(x, y)] = ("C", "crew")
            elif hip[1] - 3 <= y <= hip[1] - 1:  # belt at the seat
                cv.px[(x, y)] = ("t", "crew")
        cv.put(hip[0] + 1, hip[1] - 2, "D", "crew"); cv.put(hip[0] + 2, hip[1] - 2, "D", "crew")
        cv.paint(disc((sh[0] + 1, sh[1] - 2), 2.3), "crew", ("p", "R", "R"))  # scarf wrap
        # arm to the grip
        grip = P(83, 25 + p["driver_arm"])
        elbow = ((sh[0] + grip[0]) / 2 - 1, (sh[1] + grip[1]) / 2 + 3)
        self.limb(sh, elbow, 1.6, "crew", SKIN)
        self.limb(elbow, grip, 1.4, "crew", SKIN)
        self.hand(grip, "crew")
        self.head(hd, "crew", True, p["mouth"], p["eyes"])
        self.helmet(hd, "crew")

    def sidecar(self):
        """The sidecar rider and the spiked club; the tub is drawn over them."""
        p, cv, P = self.p, self.cv, self.P
        rx, ry = p["rider"]
        # rider (behind the tub): torso, arm, head, club
        base = P(36 + rx, 38 + ry)
        sh = (base[0], base[1] - 6)
        hd = P(36 + rx + p["rider_head"][0], 27 + ry + p["rider_head"][1])
        self.limb(base, sh, 3.4, "crew", JACKET)
        hand = (sh[0] + 4 + p["reach"], sh[1] - 1 + p["rider_arm"])
        self.limb(sh, hand, 1.5, "crew", SKIN)
        self.hand(hand, "crew")
        self._hand, self._head = hand, hd

    def club(self):
        p, cv = self.p, self.cv
        hand = self._hand
        ang, ext = p["club"]
        a = math.radians(ang)
        tip = (hand[0] + 13 * math.cos(a), hand[1] - 13 * math.sin(a))
        head_c = (hand[0] + 17 * math.cos(a), hand[1] - 17 * math.sin(a))
        cv.paint(capsule(hand, tip, 1.2), "weapon", ("K", "c", "u"))
        cv.paint(disc(head_c, 3.6), "weapon", ("i", "i", "z"))
        for n in range(6):
            sa = math.radians(n * 60 + 30 + ang)
            cv.put(head_c[0] + 5 * math.cos(sa), head_c[1] - 5 * math.sin(sa), "D", "weapon")
            cv.put(head_c[0] + 4 * math.cos(sa), head_c[1] - 4 * math.sin(sa), "D", "weapon")
        self.hand(hand, "crew")
        # the rider's face stays in front of the club handle so his snarl reads at 1x
        self.head(self._head, "crew", False, p["mouth"] if p["mouth"] != "snarl" else "open", p["eyes"], ear_dx=-8, r=5.3)
        self._ball = head_c

    def tub(self):
        """Tub, skull plate, rope, strut, exhaust stubs and the sidecar wheel."""
        p, cv, P = self.p, self.cv, self.P
        hull = cv.paint(poly(self.Ps([(13, 37), (20, 40), (56, 40), (59, 47), (54, 57), (22, 58), (14, 52), (12, 43)])), "machine", PANEL)
        for (x, y) in hull:  # grime: the bottom two rows of the hull are dark
            if (x, y + 2) not in hull:
                cv.px[(x, y)] = ("p", "machine")
        for c in ((17, 47), (27, 53), (45, 54), (51, 45), (25, 44), (42, 47)):  # rust patches
            cx, cy = P(*c)
            for (x, y) in disc((cx, cy), 1.5):
                if (x, y) in hull:
                    cv.px[(x, y)] = ("c", "machine")
            cv.put(cx + 1, cy - 1, "C", "machine"); cv.put(cx + 2, cy - 1, "C", "machine")
        for ry in (43, 54):  # rivet rows: dark pairs
            for rx in range(16, 56, 5):
                if 31 <= rx <= 42 and ry == 43:
                    continue
                for d in (0, 1):
                    x, y = P(rx + d, ry)
                    if (int(round(x)), int(round(y))) in hull:
                        cv.put(x, y, "p", "machine")
        self.bands(cv.paint(chain(self.Ps([(13, 37), (20, 40), (55, 40)]), 1.2), "machine", "C"), (24, 41, 52))
        for yy, row in enumerate(SKULL):
            for xx, ch in enumerate(row):
                if ch != ".":
                    x, y = P(33 + xx + (7 - len(row)) // 2, 44 + yy)
                    cv.put(x, y, ch, "machine")
        for a, b in (((20, 39), (27, 57)), ((47, 39), (52, 56))):  # rope lashings: 2 px bands, shaded underside
            rope = capsule(P(*a), P(*b), 0.8)
            for (x, y) in rope:
                cv.put(x, y, "t" if (x - 1, y) in rope else "u", "machine")
        cv.paint(chain(self.Ps([(55, 50), (63, 49)]), 1.0), "machine", COPPER)
        for yy in (44, 48):
            cv.paint(poly(self.Ps([(9, yy), (14, yy), (14, yy + 3), (9, yy + 3)])), "machine", "C")
        if not p["wreck"]:
            self.wheel((26, 56), 7, p["spoke"] + 0.9)

    def sparks(self):
        cv = self.cv
        if self.p["debris"]:
            cv.paint(disc((106, 61), 1.8), "machine", ("c", "C", "D"))
            cv.put(106, 61, "K", "machine")
        if self.p["burst"]:
            bx, by = self.p["burst"]
            for dx, dy in ((0, 0), (1, -1), (2, -1), (3, -2), (2, -3), (3, -4), (1, 0)):
                cv.put(bx + dx, by + dy, "Y", "machine")
            for dx, dy in ((3, 0), (4, 0), (4, -1), (5, -2), (5, -3), (4, -5), (5, -5)):
                cv.put(bx + dx, by + dy, "f", "machine")
            for dx, dy in ((-21, 0), (-20, 0), (-20, -1), (-19, 0)):
                cv.put(bx + dx, by + dy, "z", "machine")
        for (x, y) in self.p["sparks"]:
            px, py = self.P(x, y)
            self.cv.put(px, py, "Y", "machine")
            self.cv.put(px + 1, py + 1, "f", "machine")

    def render(self):
        # depth order, far to near: sidecar rider, tub (on the far side of the bike), bike, club, driver
        self.sidecar()
        self.tub()
        self.machine()
        self.club()
        self.driver()
        self.sparks()
        return self.cv.cels()


POSES = {
    # idle: engine ticking over, club on the shoulder, firebox glowing low
    0: dict(glow=1, smoke=1, club=(145, 0), mouth="snarl"),
    # move contact: nose lowered, driver flat to the bars, scarf streaming, tub squatting
    3: dict(lean=2, dy=1, scarf=1.4, scarf_lift=1, glow=1, smoke=2, spoke=0.5, club=(110, 0), rider=(-1, 1), rider_head=(-1, 1),
            mouth="open", fork=-1),
    # tell hold: rears up on the back wheels, firebox flaring, club cocked straight up and back
    8: dict(pitch=7.0, pivot_x=26, lean=3, scarf=1.2, scarf_lift=2, glow=2, smoke=3, spoke=0.3, club=(100, 0), rider=(-1, -2),
            rider_head=(-1, -2), rider_arm=-2, mouth="open", driver_arm=-1),
    # attack impact: lunges 5 px, front slams down, driver prone, rider stands and chops the club flat forward
    # over the driver's back (the ram is the hit; the club is the read), sparks under the front tyre
    10: dict(pitch=-3.5, pivot_x=90, dx=6, lean=4, scarf=1.6, scarf_lift=0, glow=2, smoke=2, puff_back=True, spoke=1.1,
             club=(22, 0), rider=(8, -4), rider_head=(7, -2), rider_arm=-6, reach=10, mouth="open", driver_arm=1, burst=(101, 62)),
    # hurt: knocked back, nose popping, white eyes, O mouths, club flung up and back
    11: dict(pitch=3.0, dx=-3, lean=-2, scarf=0.9, scarf_lift=3, glow=1, smoke=1, spoke=0.2, club=(150, 0), rider=(-2, 0),
             rider_head=(-3, -1), mouth="O", eyes="w", head=(-2, -1), sparks=((97, 36), (89, 34))),
    # defeat: the wreck settles where it stopped: tyres burst flat, hull on the floor, fire out, a dark wisp,
    # the driver slumped over the bars and the rider slumped over the rim with the club hanging from his hand
    15: dict(wreck=True, dy=4, shear=-0.06, debris=True, glow=0, smoke=1, spoke=0.7, lean=5, head=(3, 5), driver_arm=5, scarf=0.6, scarf_lift=-3,
             club=(-70, 0), rider=(2, 4), rider_head=(4, 6), rider_arm=4, mouth="O", eyes="w"),
}


def render(frame: int) -> dict[str, list[str]]:
    return Rig(POSES[COPIES.get(frame, frame)]).render()


# ------------------------------------------------------------------ grid and manifest

def palette_json():
    out = []
    for key, name, rgba, accent in PALETTE:
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def build_grid() -> dict:
    cache = {k: render(k) for k in KEY_FRAMES}
    frames = []
    for n, t in enumerate(TICKS):
        cels = cache[COPIES.get(n, n)]
        frames.append({"duration_ms": round((t if t else 6) * 1000 / TICK_HZ), "cels": {k: list(v) for k, v in cels.items()}})
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(),
        "layers": list(LAYERS),
        "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def build_manifest() -> dict:
    tag_of = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
    frames = [{"index": n, "tag": tag_of[n], "ticks": t, "duration_ms": round((t if t else 6) * 1000 / TICK_HZ),
               "author": "key" if n in KEY_FRAMES else "copy"} for n, t in enumerate(TICKS)]
    return {
        "schema_version": 1,
        "id": "goblin_copper_chopper_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/goblin/chopper/chop_01.ase",
        "grid_source": "assets/source/characters/goblin/chopper/chop_01.grid.json",
        "output_atlas": "generated/art/chop_01.png",
        "output_metadata": "generated/art/chop_01.json",
        "palette_id": "chop-floor01-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Goblin Copper Chopper; runtime role IP-neutral fast light vehicle ram)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("CHOP-01 design-edit keys (atlas frames 0, 3, 8, 10, 11, 15; fixture tests/fixtures/chop-01_keys.json), "
                       "rasterized by scripts/author_chop01.py from one parametric rig; in-between slots are copies of the nearest key; "
                       "layers machine, crew, weapon"),
            "generator": "Draft spec and keys: Claude Sonnet. Design edit (keys 0, 3, 8, 10, 11, 15 redrawn and frozen): Claude Opus. Review and in-betweens pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-goblin-copper-chopper-concept-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "chopper": {
            "status": "Design edit: 6 keys frozen, 10 in-between slots are copies; silhouette gate and review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/chop-01-spec.md",
            "opaque_height_idle": 52,
            "body_height": 47,
            "length": 95,
            "crew": "baked into the vehicle frames (crew layer); no separate crew sprites",
            "tick_hz": TICK_HZ,
            "hold_preview_ms": 100,
            "grounded_tags": GROUNDED_TAGS,
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": list(KEY_FRAMES),
            "inbetween_frames": list(INBETWEENS),
            "key_fixture": "tests/fixtures/chop-01_keys.json",
            "attack_ticks": {"tell": [0, 29], "swing_smear": [30, 32], "swing_impact": [33, 41], "hitbox_active": [33, 38]},
            "frames": frames,
        },
    }


def dump(data: dict) -> str:
    data = {k: v for k, v in data.items() if not k.startswith("_")}
    return json.dumps(data, indent=1) + "\n"


def report(cels: dict, label: str) -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    import pixel_grid_to_ase as pg
    flat = [["."] * CANVAS[0] for _ in range(CANVAS[1])]
    for rows in cels.values():
        for y, r in enumerate(rows):
            for x, ch in enumerate(r):
                if ch != ".":
                    flat[y][x] = ch
    orphans, isolated = pg._stray_pixels(flat, ACCENTS)
    pts = [(x, y) for y, r in enumerate(flat) for x, ch in enumerate(r) if ch != "."]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    used = sorted({ch for r in flat for ch in r} - {"."})
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, w {max(xs) - min(xs) + 1}) "
          f"orphans {len(orphans)} isolated {len(isolated)} keys {''.join(used)}")


def preview(grid: dict, out: Path, k: int = 4) -> None:
    """Keys 0, 3, 8 (top row) and 10, 11, 15 (bottom row) at k x, alternating plum and brick-brown cells."""
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}
    w, h = CANVAS
    sheet = Image.new("RGBA", (w * k * 3, h * k * 2), (0, 0, 0, 255))
    for n, f in enumerate(KEY_FRAMES):
        bg = Image.new("RGBA", (w * k, h * k), (92, 52, 38, 255) if n % 2 else (60, 44, 58, 255))
        im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        for layer in LAYERS:
            for y, row in enumerate(grid["frames"][f]["cels"][layer]):
                for x, ch in enumerate(row):
                    if ch != ".":
                        im.putpixel((x, y), pal[ch])
        bg.alpha_composite(im.resize((w * k, h * k), Image.NEAREST))
        sheet.paste(bg, ((n % 3) * w * k, (n // 3) * h * k))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)


def scale_evidence(out: Path) -> None:
    """Key 0 at 1x on lit brick beside Carl and a GOB-01 goblin (left), keys 10 and 15 at 1x on lit brick (right), 4x crop below."""
    sys.path.insert(0, str(ROOT / "scripts"))
    import make_goblin_gob01_review as rv
    from PIL import Image
    floor = rv.Floor()
    carl_f, carl_p, _ = rv.grid_frames(rv.CARL)
    gob_f, gob_p, _ = rv.grid_frames(rv.GOBLIN_KEYS["A"])
    chop_f, chop_p, _ = rv.grid_frames(str(GRID.relative_to(ROOT)))
    gob_img, gob_pv = rv.flip(gob_f[0], gob_p)
    chop_img, chop_pv = rv.flip(chop_f[0], chop_p)
    chop = {n: rv.flip(chop_f[n], chop_p) for n in (0, 10, 15)}
    entries = [(carl_f[0], carl_p, 345, 0), (gob_img, gob_pv, 385, 0), (chop[0][0], chop[0][1], 470, 0)]
    one = floor.render(entries, 1)
    four = floor.render(entries, 4)
    view = list(floor.view)
    floor.view = [view[0] + 320] + view[1:]  # lit brick right of the archway, torch at x 920
    two = floor.render([(chop[10][0], chop[10][1], 862, 0), (chop[15][0], chop[15][1], 975, 0)], 1)
    floor.view = view
    g = floor.ground(345) - floor.view[1]
    x0, _ = floor.view_xy(325, 0)
    x1, _ = floor.view_xy(530, 0)
    crop = four.crop((x0 * 4, (g - 60) * 4, x1 * 4, (g + 8) * 4))
    sheet = Image.new("RGB", (one.width + two.width + 8, one.height + crop.height + 8), (24, 23, 34))
    sheet.paste(one, (0, 0))
    sheet.paste(two, (one.width + 8, 0))
    sheet.paste(crop, (0, one.height + 8))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {out}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "scale", "sheet"))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--preview", type=Path)
    ap.add_argument("--out", type=Path, default=SCALE_PNG)
    a = ap.parse_args(argv)
    if a.mode == "scale":
        scale_evidence(a.out)
        return 0
    if a.mode == "sheet":
        preview(json.loads(GRID.read_text()), KEYS_PNG, k=3)
        print(f"wrote {KEYS_PNG.relative_to(ROOT)}")
        return 0
    data = build_grid()
    for f in KEY_FRAMES:
        report(data["frames"][f]["cels"], f"key {f:2d}")
    if a.preview:
        preview(data, a.preview)
    text, manifest_text = dump(data), json.dumps(build_manifest(), indent=2) + "\n"
    if a.check:
        bad = []
        if not GRID.is_file() or GRID.read_text() != text:
            bad.append("grid")
        if not MANIFEST.is_file() or MANIFEST.read_text() != manifest_text:
            bad.append("manifest")
        if bad:
            print(f"differs: {bad}")
            return 1
        print("up to date")
        return 0
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text)
    MANIFEST.write_text(manifest_text)
    print(f"wrote {GRID.relative_to(ROOT)} and {MANIFEST.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
