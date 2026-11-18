"""ANIM-INTERACT-01: Carl and Donut interact set (frozen keys and drawn in-betweens), docs/02-art/interact-anim-01-spec.md.

The shared toolkit validators run with ``python3 -m scripts.pixelart validate <config>`` on
``assets/source/characters/{carl/carl_h52_interact,donut/donut_h40_interact}.pixelart.json``; this file covers the
spec rules the toolkit does not (frame and pivot parity with locomotion, contact ticks, identity, contact boxes).
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_interact01 as ai  # noqa: E402

CARL_LOCO = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
DONUT_LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
CONFIGS = [ROOT / "assets/source/characters/carl/carl_h52_interact.pixelart.json",
           ROOT / "assets/source/characters/donut/donut_h40_interact.pixelart.json"]


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def layer_pix(frame: dict, layer: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(layer, [])) for x, c in enumerate(r) if c != "."}


def flat(grid: dict, i: int) -> dict:
    out = {}
    for l in grid["layers"]:
        out.update(layer_pix(grid["frames"][i], l))
    return out


def stamp(pix: dict, w: int, h: int) -> tuple[int, int]:
    """Top-left of the fist stamp (``w`` x ``h``) at the free end of an arm: the right-most column, lowest row."""
    x0 = max(x for x, _ in pix) - (w - 1)
    return x0, max(y for x, y in pix if x >= x0) - (h - 1)


def translation_of(a: dict, b: dict):
    """(dx, dy) if pixel set ``a`` is ``b`` translated exactly, else None."""
    if not a or len(a) != len(b):
        return None
    ax, ay = min(a)
    bx, by = min(b)
    dx, dy = ax - bx, ay - by
    return (dx, dy) if all(a.get((x + dx, y + dy)) == c for (x, y), c in b.items()) else None


# Spec section 5 in-between table: Carl's upper-block translation U(dx, dy) and head offset per frame (keys included).
CARL_U = {0: ((1, 0), (0, 0)), 1: ((2, 1), (0, 0)), 2: ((-1, 3), (0, 1)), 3: ((0, 2), (0, 0)),
          4: ((1, 5), (0, 0)), 5: ((1, 11), (1, 0)), 6: ((1, 6), (1, 0)), 7: ((1, 1), (0, 0)),
          8: ((4, 2), (0, 0)), 9: ((8, 3), (1, 1)), 10: ((8, 3), (1, 2)), 11: ((3, 1), (0, 0))}
# Donut head, face and tiara translation (dx, dy) per frame (n2 differs from the table, see the spec).
DONUT_HEAD = {0: (0, 1), 1: (1, 1), 2: (1, 1), 3: (0, 0), 4: (1, 5), 5: (2, 10), 6: (2, 11), 7: (1, 4)}


class InteractGridTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.carl, cls.donut = load(ai.CARL_OUT), load(ai.DONUT_OUT)
        cls.carl_loco, cls.donut_loco = load(CARL_LOCO), load(DONUT_LOCO)

    def test_committed_grids_match_the_generator(self):
        for path, text in ai.outputs().items():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)

    def test_same_frame_pivot_palette_and_layers_as_locomotion(self):
        for g, loco in ((self.carl, self.carl_loco), (self.donut, self.donut_loco)):
            for k in ("canvas", "pivot", "palette", "layers"):
                self.assertEqual(g[k], loco[k], k)
        self.assertEqual(self.carl["canvas"], [48, 56])
        self.assertEqual(self.carl["pivot"], [25, 55])
        self.assertEqual(self.donut["canvas"], [72, 44])
        self.assertEqual(self.donut["pivot"], [46, 43])

    def test_tags_frames_and_tick_durations(self):
        for g, plan in ((self.carl, ai.CARL_PLAN), (self.donut, ai.DONUT_PLAN)):
            self.assertEqual([t["name"] for t in g["tags"]], [p[0] for p in plan])
            i = 0
            for tag, (name, rows, _) in zip(g["tags"], plan):
                self.assertEqual((tag["from"], tag["to"]), (i, i + len(rows) - 1), name)
                for _, ticks, _ in rows:
                    self.assertEqual(g["frames"][i]["duration_ms"], round(ticks * 1000 / 60))
                    i += 1
            self.assertEqual(len(g["frames"]), i)

    def test_contact_ticks(self):
        self.assertEqual(ai.contact_ticks(ai.CARL_PLAN),
                         {"interact_hand": 5, "interact_floor": 8, "interact_push": 6})
        self.assertEqual(ai.contact_ticks(ai.DONUT_PLAN), {"interact_paw": 6, "interact_nudge": 8})

    def test_config_ticks_match_the_grid(self):
        for cfg_path, g in zip(CONFIGS, (self.carl, self.donut)):
            cfg = load(cfg_path)
            for tag in g["tags"]:
                rows = cfg["tags"][tag["name"]]["frames"]
                self.assertEqual([r[0] for r in rows], list(range(tag["from"], tag["to"] + 1)))
                for idx, ticks in rows:
                    self.assertEqual(g["frames"][idx]["duration_ms"], round(ticks * 1000 / 60))

    def test_grounded_and_clipping(self):
        for g in (self.carl, self.donut):
            w, h = g["canvas"]
            for i in range(len(g["frames"])):
                pix = flat(g, i)
                self.assertEqual(max(y for _, y in pix), g["pivot"][1], f"frame {i} lowest row")
                self.assertFalse([p for p in pix if p[0] in (0, w - 1) or p[1] == 0], f"frame {i} touches the edge")

    def test_carl_identity_parts_are_translated_master_pixels(self):
        """Every frame, keys and in-betweens: torso, jacket, pin and hair are the master's pixels, translated only."""
        idle = self.carl_loco["frames"][0]
        for i in range(12):
            fr = self.carl["frames"][i]
            for layer in ("jacket", "torso", "pin", "hair"):
                self.assertIsNotNone(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), (i, layer))

    def test_donut_identity_parts_are_translated_master_pixels(self):
        idle = self.donut_loco["frames"][0]
        for i in range(8):
            fr = self.donut["frames"][i]
            for layer in ("head", "face"):
                self.assertIsNotNone(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), (i, layer))
            tail = layer_pix(fr, "tail")
            if i == 6:      # n2: the tail tip is stretched 1 px up, everything from row 15 down is the master tail
                master = layer_pix(idle, "tail")
                self.assertEqual({k: v for k, v in tail.items() if k[1] >= 15}, {k: v for k, v in master.items() if k[1] >= 15})
                self.assertEqual(min(y for _, y in tail), min(y for _, y in master) - 1)
            else:
                self.assertIsNotNone(translation_of(tail, layer_pix(idle, "tail")), (i, "tail"))

    def test_contact_boxes(self):
        """Spec section 4: the contact part reaches its object-height class (pivot-relative, y up)."""
        px, py = self.carl["pivot"]

        def reach(g, i, layer):
            pts = layer_pix(g["frames"][i], layer)
            right = max(x for x, _ in pts)
            ys = [py - y for x, y in pts if x >= right - 4]
            return right - px, min(ys), max(ys)

        seat = layer_pix(self.carl["frames"][5], "boxers")
        self.assertLessEqual(max(y for _, y in seat), py - 4)   # p1 seat clear of the floor (not sitting)

        r, lo, hi = reach(self.carl, 1, "far_arm")      # u1 grip at hand height
        self.assertTrue(16 <= r <= 18 and lo >= 24 and hi <= 33, (r, lo, hi))
        r, lo, hi = reach(self.carl, 5, "far_arm")      # p1 hand on a floor item
        self.assertTrue(r >= 18 and lo <= 6, (r, lo, hi))
        r, lo, hi = reach(self.carl, 9, "near_arm")     # s1 fists on the prop face
        self.assertTrue(r >= 19 and 16 <= lo <= 26, (r, lo, hi))
        r2, lo2, _ = reach(self.carl, 9, "far_arm")     # s1: the second fist sits higher (two contact heights)
        self.assertGreaterEqual(lo2 - lo, 4)
        dpx, dpy = self.donut["pivot"]
        paw = layer_pix(self.donut["frames"][1], "legs_near")
        self.assertGreaterEqual(max(x for x, _ in paw) - dpx, 20)
        self.assertLessEqual(dpy - max(y for _, y in paw), 6)

    def test_carl_upper_block_and_head_follow_the_in_between_table(self):
        """The spec table: torso and jacket move by U, hair by U + head offset, in every frame."""
        idle = self.carl_loco["frames"][0]
        for i, ((ux, uy), (hx, hy)) in CARL_U.items():
            fr = self.carl["frames"][i]
            for layer in ("torso", "jacket"):
                self.assertEqual(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), (ux, uy), (i, layer))
            self.assertEqual(translation_of(layer_pix(fr, "hair"), layer_pix(idle, "hair")), (ux + hx, uy + hy), i)
            self.assertLessEqual(max(abs(hx), abs(hy)), 2, i)           # head offset at most 2 px

    def test_donut_head_follows_the_in_between_table(self):
        idle = self.donut_loco["frames"][0]
        for i, d in DONUT_HEAD.items():
            fr = self.donut["frames"][i]
            for layer in ("head", "face"):
                self.assertEqual(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), d, (i, layer))

    def test_carl_floor_pickup_never_reads_as_sitting(self):
        """p0, p1 and p2: the boxer seat stays at least 4 px off the floor (spec section 5)."""
        for i in (4, 5, 6):
            seat = layer_pix(self.carl["frames"][i], "boxers")
            self.assertLessEqual(max(y for _, y in seat), self.carl["pivot"][1] - 4, i)

    def test_carl_pickup_hand_anchor_matches_the_config(self):
        """The carried-item anchor is the far fist's 6 x 4 stamp, third column and third row (pivot-relative, y up)."""
        cfg = load(CONFIGS[0])
        px, py = self.carl["pivot"]
        anchors = cfg["anchors"]["carried_item_hand"]
        self.assertEqual(anchors["5"], [16, 5])                    # the spec value at p1
        for frame, (ax, ay) in anchors.items():
            fist = layer_pix(self.carl["frames"][int(frame)], "far_arm")
            x0, y0 = stamp(fist, 6, 4)
            self.assertEqual((x0 + 2 - px, py - (y0 + 2)), (ax, ay), frame)

    def test_carl_hand_verb_in_between_poses(self):
        c = self.carl["frames"]

        def far_right(i):
            return max(x for x, _ in layer_pix(c[i], "far_arm"))

        # u0: elbow bent, the far fist is lower and nearer than the u1 grip, rows 30-33
        x0, y0 = stamp(layer_pix(c[0], "far_arm"), 6, 4)
        self.assertEqual((x0, y0), (34, 30))                       # x34-39, rows 30-33
        self.assertLess(far_right(0), far_right(1))
        # u3 lets go: the far arm is the master idle far arm, translated by U(0, +2)
        idle = self.carl_loco["frames"][0]
        self.assertEqual(translation_of(layer_pix(c[3], "far_arm"), layer_pix(idle, "far_arm")), (0, 2))

    def test_carl_push_in_between_poses(self):
        c = self.carl["frames"]
        # s0: fists already at two heights (chest and belly), the back foot is behind the s1 back foot
        far, near = stamp(layer_pix(c[8], "far_arm"), 6, 6), stamp(layer_pix(c[8], "near_arm"), 6, 6)
        self.assertGreaterEqual(near[1] - far[1], 4)               # chest fist 4+ px above the belly fist
        self.assertLess(near[0], far[0] + 3)
        self.assertLess(min(x for x, _ in layer_pix(c[8], "shoes")), min(x for x, _ in layer_pix(c[9], "shoes")))
        # s2 holds the contact: arms are s1's pixels, the head is 1 px lower, the back heel 1 px lower
        for layer in ("far_arm", "near_arm", "watch"):
            self.assertEqual(layer_pix(c[10], layer), layer_pix(c[9], layer), layer)
        self.assertEqual(translation_of(layer_pix(c[10], "hair"), layer_pix(c[9], "hair")), (0, 1))
        back1 = [y for (x, y) in layer_pix(c[9], "shoes") if x <= 18]
        back2 = [y for (x, y) in layer_pix(c[10], "shoes") if x <= 18]
        self.assertEqual((min(back2), max(back2)), (min(back1) + 1, 55))
        # s3: arms back at idle (the master far and near arms, translated), grounded
        idle = self.carl_loco["frames"][0]
        for layer in ("far_arm", "near_arm"):
            self.assertEqual(translation_of(layer_pix(c[11], layer), layer_pix(idle, layer)), (3, 1), layer)

    def test_carl_feet_stay_planted(self):
        """No root motion: within a verb neither Croc's left edge jumps more than 4 px between frames."""
        for tag in self.carl["tags"]:
            lefts = []
            for i in range(tag["from"], tag["to"] + 1):
                xs = sorted({x for x, _ in layer_pix(self.carl["frames"][i], "shoes")})
                gap = [b for a, b in zip(xs, xs[1:]) if b - a > 1]
                lefts.append((xs[0], gap[0] if gap else xs[-1]))
            for a, b in zip(lefts, lefts[1:]):
                self.assertLessEqual(max(abs(a[0] - b[0]), abs(a[1] - b[1])), 4, (tag["name"], lefts))

    def test_donut_cat_actions_only(self):
        """Only the near foreleg, head and tail change: the far legs stay, the body differs from idle only where the
        near foreleg lifted off, and the near leg is the master's in the nudge frames."""
        idle = self.donut_loco["frames"][0]
        for i in range(8):
            fr = self.donut["frames"][i]
            self.assertEqual(layer_pix(fr, "legs_far"), layer_pix(idle, "legs_far"), i)
            body_i, body_idle = layer_pix(fr, "body"), layer_pix(idle, "body")
            changed = {k for k in set(body_i) | set(body_idle) if body_i.get(k) != body_idle.get(k)}
            self.assertTrue(all(53 <= x <= 61 and y >= 41 for x, y in changed), (i, sorted(changed)[:5]))
            if i not in (0, 1, 2):
                self.assertEqual(layer_pix(fr, "legs_near"), layer_pix(idle, "legs_near"), i)
        for i in (4, 5, 6, 7):
            self.assertEqual(layer_pix(self.donut["frames"][i], "body"), layer_pix(idle, "body"), i)

    def test_donut_paw_in_between_poses(self):
        d = self.donut["frames"]

        def paw(i):
            return layer_pix(d[i], "legs_near")

        # d0: tucked round paw at rows 30-34, x58-62, above the d1 paw (rows 34-38)
        far0 = [(x, y) for (x, y) in paw(0) if x >= 58]
        self.assertEqual((min(x for x, _ in far0), max(x for x, _ in far0)), (58, 62))
        self.assertEqual((min(y for _, y in far0), max(y for _, y in far0)), (30, 34))
        # d2: the d1 paw 2 px higher, same x reach (a lift, not a swing)
        self.assertEqual(max(x for x, _ in paw(2)), max(x for x, _ in paw(1)))
        self.assertEqual(max(y for _, y in paw(1)) - max(y for _, y in paw(2)), 2)
        # d3: the master idle pose
        self.assertEqual(d[3]["cels"], self.donut_loco["frames"][0]["cels"])

    def test_donut_nudge_never_touches_the_right_edge(self):
        """n2 uses head (+2, +11) instead of the table's (+3, +10): +3 would put the head outline on column W-1."""
        self.assertLess(max(x for x, _ in flat(self.donut, 6)), self.donut["canvas"][0] - 1)

    def test_manifests_match_config_and_grid(self):
        """The manifest ``interact.moves`` tick contract equals the config tick table, the grid tags and the plans."""
        for cfg_path, g, plan in zip(CONFIGS, (self.carl, self.donut), (ai.CARL_PLAN, ai.DONUT_PLAN)):
            cfg = load(cfg_path)
            man = load(ROOT / cfg["manifest"])
            self.assertEqual(man["frame_size"], g["canvas"])
            self.assertEqual(man["pivot"], g["pivot"])
            self.assertEqual(man["animation_tags"], [t["name"] for t in g["tags"]])
            for name, tag in cfg["tags"].items():
                move = man["interact"]["moves"][name]
                self.assertEqual(move["total_ticks"], sum(t for _, t in tag["frames"]), name)
                t0 = 0
                for (_, ticks), mf in zip(tag["frames"], move["frames"]):
                    self.assertEqual(mf["ticks"], [t0, t0 + ticks - 1], (name, mf["name"]))
                    t0 += ticks
                contact = [mf for mf in move["frames"] if mf["phase"] == "contact"]
                self.assertEqual(len(contact), 1)
                self.assertEqual(move["contact_tick"], contact[0]["ticks"][0], name)
            self.assertEqual({k: v["contact_tick"] for k, v in man["interact"]["moves"].items()}, ai.contact_ticks(plan))

    def test_evidence_checksums_match(self):
        import hashlib
        ev = ROOT / "docs/evidence/anim-interact-01"
        rows = [ln.split("  ", 1) for ln in (ev / "SHA256SUMS").read_text(encoding="utf-8").splitlines() if ln]
        self.assertGreaterEqual(len(rows), 20)
        for digest, name in rows:
            self.assertEqual(hashlib.sha256((ev / name).read_bytes()).hexdigest(), digest, name)
        listed = {name for _, name in rows}
        on_disk = {str(p.relative_to(ev)) for p in ev.rglob("*") if p.is_file() and p.name != "SHA256SUMS"}
        self.assertEqual(listed, on_disk)

    def test_toolkit_validators_pass(self):
        for cfg in CONFIGS:
            out = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(cfg)], cwd=ROOT,
                                 capture_output=True, text=True)
            self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
            self.assertIn("verdict: PASS", out.stdout)


if __name__ == "__main__":
    unittest.main()
