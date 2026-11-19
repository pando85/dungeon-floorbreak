#!/usr/bin/env python3
"""GOB-02B-DEATH authoring record: goblin variant B death poses and death-parts sheet (pixel-grid-v1 sources).

Usage:

    python3 scripts/author_gob02b_death.py keys [--check]
    python3 scripts/author_gob02b_death.py manifests

Variant B is a palette and feature variant of the approved goblin A death set (``docs/02-art/gob-02-death-spec.md``),
exactly as GOB-01 variant B is of GOB-01 A. ``keys`` writes the two B sources from the committed A sources plus one new
layer:

* ``body``, ``weapon`` and ``gore`` are the A cels, byte for byte (tested): same frames, frame sizes, pivots, anchors,
  tick windows, part ids and gore layering, so the cosmetic spawner reads B with the A data contract unchanged;
* ``accessory`` (between ``weapon`` and ``gore``) carries the GOB-01 B identity cues: the red rag bandana (2-row
  ``R``/``B`` band, knot at the back of the skull, two tails), the ear notch (2 ``K``), the missing front upper tooth
  (``o``) and the 3 x 3 stitched vest patch (``uuu`` / ``uKu`` / ``uKu``).

Where each accessory overlay comes from (``docs/02-art/gob-02b-death-spec.md`` section 3):

* pose 0 ``crush_contact``: new literal rows (the bandana pressed flat under the stone, a top notch in the flattened
  ear, the front tooth gone, the patch on the squashed vest);
* pose 1: pose 0's overlay moved by the A flatten rule (rows 33 to 45 down 3);
* pose 2 ``crushed_remains``: GOB-01 B frame 25's head overlay (bandana, tails, tooth), mirrored like the A heap
  (x' = 59 - x), plus the patch on the rag mass;
* pose 3 (headless): only the vest patch of GOB-01 B frame 16 (the bandana leaves with the head); pose 5: the same
  patch as literal rows on the corpse's rag mass (x 32 to 34, rows 51 to 53), inside A's silhouette;
* pose 4: pose 3's patch moved by the A buckle rule (down 4, left 2);
* parts ``head`` and ``head_dead``: GOB-01 B frame 0's head overlay through the A head mask and shift; ``torso_upper``:
  frame 0's vest patch; ``ear``: frame 0's ear notch. Every other part has an empty accessory layer.

No concept pixels, Death Lab pixels or image-model output; nothing is wired into the game.
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_gob02_death as ad  # noqa: E402

DEATH_DIR = ad.DEATH_DIR
POSES = DEATH_DIR / "goblin_b_death_h36.grid.json"
POSES_ASE = DEATH_DIR / "goblin_b_death_h36.ase"
PARTS = DEATH_DIR / "goblin_b_death_parts.grid.json"
PARTS_ASE = DEATH_DIR / "goblin_b_death_parts.ase"
GOBLIN_B = ROOT / "assets/source/characters/goblin/goblin_b_h36.grid.json"
LAYERS = ["body", "weapon", "accessory", "gore"]
ACCESSORY_KEYS = set("KRBuoltsma")        # bandana R/B, outline K, tooth o, patch u; body-coloured tidy pixels
SPEC = "docs/02-art/gob-02b-death-spec.md"

Pix = dict


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def a_poses() -> dict:
    return load(ad.POSES)


def a_parts() -> dict:
    return load(ad.PARTS)


def b_acc(frame: int) -> Pix:
    return ad.cel_pix(load(GOBLIN_B)["frames"][frame]["cels"]["accessory"])


def close_acc(acc: Pix, base: set, size: tuple[int, int] = ad.CANVAS) -> Pix:
    """Outline new overhanging accessory pixels: an empty 4-neighbour (not base, not accessory) becomes ``K``."""
    out = dict(acc)
    for (x, y), ch in acc.items():
        if ch == "K":
            continue
        for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if n not in base and n not in acc and 0 <= n[0] < size[0] and 0 <= n[1] < size[1]:
                out[n] = "K"
    return out


# --------------------------------------------------------------------------- pose 0: crush_contact (new literal rows)
# Pose 0 head (A): stone underside on row 33; skull top x 23 to 34 on rows 34 to 37, the bulging eye at x 35 to 42,
# the ear pressed flat back over the hump (rows 34 to 37, tip at x 3), the top teeth on row 41 (front tooth x 45),
# the squashed vest on rows 39 to 50.
CRUSH_ACC = {
    34: [(7, "K"), (23, "RRRRRRRRRRRR")],         # ear notch (a break in the lit rim); band pressed under the stone
    35: [(7, "K"), (21, "RRRBBBBBBBBBBB")],       # band shadow row; the knot starts at the back of the skull
    36: [(19, "RRBB")],                           # knot and the R tail squeezed back along the ear's underside
    37: [(14, "RRRRR"), (21, "B")],               # R tail pressed flat under the ear; the B tail starts down
    38: [(21, "B")],                              # B tail hangs lower, over the vest's top edge
    39: [(21, "B")],
    41: [(14, "uuu"), (45, "o")],                 # vest patch; the missing front upper tooth
    42: [(14, "uKu")],
    43: [(14, "uKu")],
}


def crush_acc() -> Pix:
    a = a_poses()["frames"][0]["cels"]
    base = set(ad.cel_pix(a["body"])) | set(ad.cel_pix(a["weapon"]))
    return close_acc(ad.segs(CRUSH_ACC), base)


def flatten_acc() -> Pix:
    """Pose 1: pose 0's overlay moved by the A flatten rule (rows 33 to 45 down 3; rows 46 to 48 left of x 50 are
    replaced; the rest stays)."""
    out = {}
    for (x, y), ch in crush_acc().items():
        if 33 <= y <= 45:
            out[(x, y + 3)] = ch
        elif y >= 49 or (y >= 46 and x >= 50):
            out[(x, y)] = ch
    return out


REMAINS_PATCH = {49: [(26, "uuu")], 50: [(26, "uKu")], 51: [(26, "uKu")]}     # on the rag mass, final coordinates
REMAINS_BAND_END = 52                   # the band stops here; below it the pressed-out seep starts (review fix E1)
REMAINS_CAP = {REMAINS_BAND_END + 1: [(48, "KK")]}   # dark outline cap under the band (x 48 and 49), body pixels beneath


def remains_acc() -> Pix:
    """Pose 2: GOB-01 B frame 25's head overlay (the A heap reuses frame 25's head left of x 18/22), mirrored with
    the A heap rule x' = 59 - x, plus the patch on the pressed rag mass."""
    head = {(x, y): ch for (x, y), ch in b_acc(25).items() if x <= (22 if y <= 50 else 18)}
    mirrored = {(ad.REMAINS_MIRROR_X - x, y): ch for (x, y), ch in head.items() if y <= REMAINS_BAND_END}
    return ad.overlay(ad.overlay(mirrored, ad.segs(REMAINS_PATCH)), ad.segs(REMAINS_CAP))


def decap_acc() -> Pix:
    """Pose 3: GOB-01 B frame 16's vest patch (rows 36 and below); the head overlay leaves with the head."""
    return {(x, y): ch for (x, y), ch in b_acc(16).items() if y >= 36}


def buckle_acc() -> Pix:
    """Pose 4: pose 3's patch moved by the A buckle rule for the torso (down 4, left 2)."""
    return {(x - 2, y + 4): ch for (x, y), ch in decap_acc().items()}


HEADLESS_PATCH = {51: [(32, "uuu")], 52: [(32, "uKu")], 53: [(32, "uKu")]}   # on the dark rag mass, inside A (review fix E2)


def headless_acc() -> Pix:
    """Pose 5: the vest patch moved up onto the corpse's rag mass (x 32 to 34, rows 51 to 53), every pixel inside A's
    body silhouette. GOB-01 B frame 25 puts it on the dark rag flap on the floor row, which turned into a light peg."""
    return ad.segs(HEADLESS_PATCH)


POSE_ACC = [crush_acc, flatten_acc, remains_acc, decap_acc, buckle_acc, headless_acc]


# --------------------------------------------------------------------------- parts
def head_acc() -> Pix:
    return ad.shift(ad.mask_rows(b_acc(0), ad.HEAD_ROWS), *ad.HEAD_SHIFT)


def torso_acc() -> Pix:
    return ad.shift(ad.mask_rows(b_acc(0), ad.TORSO_ROWS), *ad.TORSO_SHIFT)


def ear_acc() -> Pix:
    ad.parts_grid()                                   # fills the A frame-0 -> cell translations
    x0, y0, x1, y1 = ad.EAR_RECT
    notch = {(x, y): ch for (x, y), ch in b_acc(0).items() if x0 <= x <= x1 and y0 <= y <= y1}
    return ad.shift(notch, *ad.CELL_SHIFT_RECORD["ear"])


PART_ACC = {"head": head_acc, "head_dead": head_acc, "torso_upper": torso_acc, "ear": ear_acc}


# --------------------------------------------------------------------------- grids
def with_accessory(a_grid: dict, overlays: list) -> dict:
    grid = copy.deepcopy(a_grid)
    grid["layers"] = LAYERS
    size = tuple(grid["canvas"])
    for frame, acc in zip(grid["frames"], overlays):
        c = frame["cels"]
        if any(ch not in ACCESSORY_KEYS for ch in acc.values()):
            raise ValueError("unexpected accessory key")
        frame["cels"] = {"body": c["body"], "weapon": c["weapon"], "accessory": ad.rows_of(acc, size),
                         "gore": c["gore"]}
    return grid


def closed_pose_acc(n: int) -> Pix:
    """Pose ``n``'s overlay, outlined where it hangs past the A body (the remains' tail ends under the gore seep, the
    corpse patch's bottom corner on the floor row), so preset 1 (gore hidden) stays a closed sprite."""
    a = a_poses()["frames"][n]["cels"]
    return close_acc(POSE_ACC[n](), set(ad.cel_pix(a["body"])) | set(ad.cel_pix(a["weapon"])))


def poses_grid() -> dict:
    return with_accessory(a_poses(), [closed_pose_acc(n) for n in range(len(POSE_ACC))])


def closed_part_acc(i: int, pid: str) -> Pix:
    """Part ``pid``'s overlay, outlined where it hangs past the A part (the gap between the two bandana tails, open in
    GOB-01 B frame 0, closes on the loose head)."""
    if pid not in PART_ACC:
        return {}
    a = a_parts()["frames"][i]["cels"]
    return close_acc(PART_ACC[pid](), set(ad.cel_pix(a["body"])) | set(ad.cel_pix(a["weapon"])), ad.CELL)


def parts_grid() -> dict:
    return with_accessory(a_parts(), [closed_part_acc(i, pid) for i, pid in enumerate(ad.PART_IDS)])


# --------------------------------------------------------------------------- manifests (the A contract, variant B)
PROVENANCE_ORIGIN = ("GOB-02B-DEATH goblin variant B death art at intensity preset 4 (" + SPEC + "): the GOB-02-DEATH A "
                     "body, weapon and gore cels unchanged plus an accessory layer (GOB-01 B bandana, ear notch, "
                     "missing tooth, vest patch) written by scripts/author_gob02b_death.py from GOB-01 B overlays and "
                     "literal rows; no image-model output, no tracing")
PROVENANCE_GEN = ("Design, overlays and evidence: Claude Opus; review fixes: Claude Sonnet. "
                  "Review pending (separate top-tier agent)")


def b_manifest(a_manifest: dict, mid: str, stem: str, grid: Path, ase: Path, section: str) -> dict:
    m = copy.deepcopy(a_manifest)
    m["id"] = mid
    m["source_path"] = str(ase.relative_to(ROOT))
    m["grid_source"] = str(grid.relative_to(ROOT))
    m["output_atlas"] = f"generated/art/{stem}.png"
    m["output_metadata"] = f"generated/art/{stem}.json"
    m["provenance"]["origin"] = PROVENANCE_ORIGIN
    m["provenance"]["generator"] = PROVENANCE_GEN
    d = m[section]
    d["spec"] = SPEC
    d["base_spec"] = "docs/02-art/gob-02-death-spec.md"
    d["variant"] = "B"
    d["layers"] = LAYERS
    d["status"] = "design stage; independent review pending; not wired into any Bevy binary"
    d["key_fixture"] = d["key_fixture"].replace("gob-02-death", "gob-02b-death")
    d["variant_of"] = str(a_manifest["grid_source"])
    return m


def pose_manifest() -> dict:
    return b_manifest(ad.pose_manifest(), "goblin_b_death_h36_candidate_v1", "goblin_b_death_h36", POSES, POSES_ASE,
                      "death_poses")


def parts_manifest() -> dict:
    ad.parts_grid()
    return b_manifest(ad.parts_manifest(), "goblin_b_death_parts_v1", "goblin_b_death_parts", PARTS, PARTS_ASE,
                      "death_parts")


def write_manifests() -> list[Path]:
    paths = []
    for name, m in (("goblin_b_death_h36.json", pose_manifest()), ("goblin_b_death_parts.json", parts_manifest())):
        path = ROOT / "assets/manifests" / name
        path.write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")
        paths.append(path)
    return paths


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "manifests"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "manifests":
        for path in write_manifests():
            print(f"wrote {path.relative_to(ROOT)}")
        return 0
    outputs = [(POSES, POSES_ASE, poses_grid()), (PARTS, PARTS_ASE, parts_grid())]
    if args.check:
        same = all(p.is_file() and p.read_text(encoding="utf-8") == ad.dump(g) for p, _a, g in outputs)
        print("B death sources up to date" if same else "B death sources differ from the generator")
        return 0 if same else 1
    import pixel_grid_to_ase as pga  # noqa: E402
    for path, ase, grid in outputs:
        path.write_text(ad.dump(grid), encoding="utf-8")
        pga.write_ase(grid, ase)
        print(f"wrote {path.relative_to(ROOT)}, {ase.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
