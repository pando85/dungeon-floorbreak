#!/usr/bin/env python3
"""BOSS-F01 evidence: contact sheets (1x, 4x), GIFs at spec timing, in-scene composites at 640 x 360, the silhouette-gate
pack (measurements only, no verdict) and the validator report, under ``docs/evidence/boss-f01/``.

Usage (needs ``bash scripts/export_assets.sh`` first for the lit Floor 1 kit atlases):

    python3 scripts/make_boss_f01_evidence.py

Everything is composed from the committed grids and the manifests' tick tables; nothing is a Bevy capture. The output is
deterministic (no timestamps). Presentation-only art tooling.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw  # noqa: E402

from scripts.pixelart import evidence, scene, validators  # noqa: E402
from scripts.pixelart.common import (CARL_X, DONUT_X, flip, grid_images, load_grid_file, load_json, sha256_file)  # noqa: E402
from scripts.pixelart.config import AssetConfig, load_config  # noqa: E402

OUT = ROOT / "docs/evidence/boss-f01"
SRC = "assets/source/enemies/brute_boss_f01"
CONFIG = ROOT / SRC / "brute_boss_f01.pixelart.json"
JUICE = ROOT / "assets/source/characters/troglodyte/juicer/juice_01.grid.json"
ATLASES = {  # manifest id -> evidence subfolder, contact-sheet contact row
    "boss-f01-vfx": "fx", "boss-f01-rack": "props", "boss-f01-strut": "props", "boss-f01-ledge": "props",
    "boss-f01-emblem": "emblem",
}


def config_from_manifest(man: dict, grid_path: Path) -> AssetConfig:
    boss = man["boss"]
    tags: dict = {}
    for f in boss["frames"]:
        tags.setdefault(f["tag"], {"playback": boss["playback"][f["tag"]], "contact": "free", "frames": []})["frames"].append([f["index"], f["ticks"]])
    return AssetConfig({"format": "pixelart-asset-v1", "asset_id": man["id"].removesuffix("_candidate_v0"), "grid": str(grid_path.relative_to(ROOT)),
                        "frame_size": man["frame_size"], "pivot": man["pivot"], "contact_row": None, "tick_hz": boss["tick_hz"],
                        "hold_ms": boss["hold_preview_ms"], "tags": tags})


def silhouette(img: Image.Image, colour=(30, 22, 36)) -> Image.Image:
    out = Image.new("RGBA", img.size, (0, 0, 0, 0))
    out.paste(colour + (255,), (0, 0), img.getchannel("A").point(lambda a: 255 if a else 0))
    return out


def gate_pack(cfg: AssetConfig, imgs: list[Image.Image], report: dict, out: Path) -> list[Path]:
    """The silhouette-gate pack: flat silhouettes of every encounter frame beside the JUICE-01 base frames they must not be
    mistaken for, the six keys at 1x on the lit floor beside Carl and Donut, and a measurements sheet. No verdict."""
    out.mkdir(parents=True, exist_ok=True)
    base = grid_images(load_grid_file(JUICE))
    base_names = {0: "idle", 3: "walk", 8: "pound tell", 10: "pound impact", 12: "hurt", 15: "down"}
    w, h = imgs[0].size
    labels = [(base[i], f"JUICE {n}") for i, n in base_names.items()] + [(im, f"{i:02d} {cfg.tag_of(i)}") for i, im in enumerate(imgs)]
    cols = 6
    rows = -(-len(labels) // cols)
    sheet = Image.new("RGB", (w * cols, (h + 14) * rows), (232, 228, 216))
    d = ImageDraw.Draw(sheet)
    for n, (img, name) in enumerate(labels):
        x, y = (n % cols) * w, (n // cols) * (h + 14)
        sil = silhouette(img)
        sheet.paste(sil, (x, y), sil)
        d.line([(x, y + h - 1), (x + w, y + h - 1)], fill=(0, 150, 190))
        d.text((x + 4, y + h), name, fill=(30, 22, 36))
    p1 = out / "silhouettes_2x.png"
    sheet.resize((sheet.width * 2, sheet.height * 2), Image.NEAREST).save(p1)

    floor = scene.Floor()
    carl, carl_piv = scene.reference_idle("carl")
    donut, donut_piv = scene.reference_idle("donut")
    keys = [i for i in sorted(evidence.key_frames(cfg))]
    pages = []
    for group in (keys[:3], keys[3:]):
        entries = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
        for n, i in enumerate(group):
            img, piv = flip(imgs[i], tuple(cfg.pivot))
            entries.append((img, piv, 590 + n * 120 + (n > 0) * 20, 0))
        pages.append(floor.render(entries, 1))
    p2 = out / "keys_lit_1x.png"
    both = Image.new("RGB", (640, 360 * 2))
    both.paste(pages[0], (0, 0)); both.paste(pages[1], (0, 360))
    both.save(p2)

    lines = ["# Silhouette-gate pack, BOSS-F01 (measurements only)", "",
             "No verdict is recorded here. The gate is run by an independent top-tier agent from `silhouettes_2x.png`, `keys_lit_1x.png` and this sheet.", "",
             "Images: `silhouettes_2x.png` (the six JUICE-01 base frames first, then the 16 encounter frames, flat, cyan line = contact row 135) and "
             "`keys_lit_1x.png` (the six keys at 1x on lit Floor 1 beside Donut and Carl, 640 x 360 each, facing left like the runtime).", "",
             "Questions from `docs/02-art/floor-01-boss-design.md` section 9:", "",
             "1. Does each key read as the same JUICE-01 creature (head, mop and ropes, ear, boils, veins, cuffs and chains, two countable fists) at 1x?",
             "2. Can any encounter frame be mistaken for the pound tell (JUICE 8) or impact (JUICE 10)? `charge_tell_hold` keeps the cocked arm behind and the knuckles ahead; `phase_roar_hold` keeps both fists below the chest line.",
             "3. Opaque heights within 4 px of the table, contact row 135, at least 1 px margin to the canvas edge (table below).",
             "4. 19 palette colours, no orphan clusters, closed outline.", "5. Thigh and shin proportions, a foot on every leg, no spike legs.",
             "6. Tone: lumpy, pitiful, grotesque; `phase_roar_hold` reads as a parody flex.", "",
             "| Frame | Name | Tag | Key | Opaque h | Spec h | Width | Top row |", "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    spec_h = json.loads((ROOT / "assets/manifests/boss-f01.json").read_text(encoding="utf-8"))["boss"]["frames"]
    import author_boss_f01_set as bs
    heights = bs.KEY_HEIGHT
    for i, im in enumerate(imgs):
        box = im.getchannel("A").getbbox()
        name = spec_h[i]["name"]
        lines.append(f"| {i} | {name} | {cfg.tag_of(i)} | {'K' if i in evidence.key_frames(cfg) else ''} | {box[3] - box[1]} | {heights.get(name, '')} | {box[2] - box[0]} | {box[1]} |")
    c = report["counts"]
    lines += ["", f"Validator report: {c['pass']} pass, {c['fail']} fail, {c['skipped']} skipped (`../validator/validator_summary.txt`).", ""]
    p3 = out / "gate_pack.md"
    p3.write_text("\n".join(lines), encoding="utf-8")
    return [p1, p2, p3]


ARENA_X0 = 328                                                      # layout x of the view's left edge: design x + 328


def _wall(img: Image.Image) -> Image.Image:
    """Cosmetic mirror-faced walls at design x 0..32 and 608..640 (the arena's crash surfaces; the real wall art is AREA-GYM-01)."""
    d = ImageDraw.Draw(img)
    for x0 in (0, 608):
        d.rectangle([x0, 0, x0 + 31, 359], fill=(38, 28, 36))
        d.rectangle([x0 + 3, 40, x0 + 28, 300], fill=(72, 82, 104), outline=(20, 14, 24))
        d.line([(x0 + 8, 60), (x0 + 22, 150)], fill=(150, 160, 180))
    return img


def arena_composite(imgs: list[Image.Image], out: Path) -> list[Path]:
    """Two lit 640 x 360 panels in the design's arena coordinates (walls 0..32 and 608..640, strut at x 200, rack over the
    footprint centre 123, the boss crashed at pivot 130, Carl at the bait x 180): the stun with the hanging rack, the
    shockwave, the mirror crack, a goblin beside Donut; and the fight's end: the landed stack on the pinned boss, and the dead
    boss with the pool."""
    floor = scene.Floor()
    carl, carl_piv = scene.reference_idle("carl")
    donut, donut_piv = scene.reference_idle("donut")
    gob, gob_piv = scene.reference_idle("goblin")
    def grid(name):
        g = load_grid_file(ROOT / SRC / f"brute_boss_f01_{name}.grid.json")
        return grid_images(g), tuple(g["pivot"])
    rack, rack_piv = grid("rack"); strut, strut_piv = grid("strut"); vfx, _vp = grid("vfx")
    cfg = load_config(CONFIG)
    bp = tuple(cfg.pivot)
    X = lambda d: ARENA_X0 + d
    stun, stun_piv = flip(imgs[9], bp)                                     # faces the left wall
    carl_l, carl_lp = flip(carl, carl_piv)
    gob_l, gob_lp = flip(gob, gob_piv)
    rip, rip_piv = flip(vfx[5], (76, 71))                                  # ripple travelling toward the left wall
    crack, crack_piv = vfx[21], (76, 24)
    panel1 = [(rack[0], rack_piv, X(123), 189, False), (strut[0], strut_piv, X(200), 0, False),
              (stun, stun_piv, X(130), 0), (carl_l, carl_lp, X(180), 0), (donut, donut_piv, X(215), 0), (gob_l, gob_lp, X(260), 0),
              (rip, rip_piv, X(420), 0, False)]
    pinned, pinned_piv = flip(imgs[14], bp)
    dead, dead_piv = flip(imgs[15], bp)
    landed, landed_piv = flip(rack[5], rack_piv)
    pool, pool_piv = vfx[34], (76, 62)
    panel2 = [(pinned, pinned_piv, X(130), 0), (landed, landed_piv, X(123), 0, False),
              (pool, pool_piv, X(480), 0, False), (dead, dead_piv, X(480), 0)]
    paths = []
    for name, entries in (("arena_640x360.png", panel1), ("arena_pinned_640x360.png", panel2)):
        path = out / name
        img = _wall(floor.render(entries, 1))
        if name == "arena_640x360.png":                                    # the crack on the left wall mirror at hump height
            ground = floor.ground(X(130)) - 20                              # screen row of the walking line (view top y 20)
            img.paste(crack.crop((46, 4, 106, 44)), (-14, ground - 82), crack.crop((46, 4, 106, 44)))
        img.save(path)
        paths.append(path)
    return paths


def main() -> int:
    cfg = load_config(CONFIG)
    grid = cfg.load_grid()
    imgs = grid_images(grid)
    OUT.mkdir(parents=True, exist_ok=True)
    written = evidence.build_evidence(cfg, OUT, grid)
    written = [p for p in written if p.name != "SHA256SUMS"]
    report = validators.run(validators.build_context(cfg))
    vdir = OUT / "validator"
    written += list(validators.write_report(report, vdir))
    written += gate_pack(cfg, imgs, report, OUT / "silhouette-gate")
    written += arena_composite(imgs, OUT)
    for mid, sub in ATLASES.items():
        man = load_json(ROOT / f"assets/manifests/{mid}.json")
        gpath = ROOT / man["grid_source"]
        c = config_from_manifest(man, gpath)
        g = load_grid_file(gpath)
        im = grid_images(g)
        d = OUT / sub / mid
        d.mkdir(parents=True, exist_ok=True)
        if mid != "boss-f01-emblem":
            written += evidence.make_gifs(c, im, d, scales=(1, 3))
        for k in (1, 4):
            p = d / f"contact_sheet_x{k}.png"
            evidence.contact_sheet(c, im, k, max_width=2400).save(p)
            written.append(p)
        if mid == "boss-f01-emblem":
            well = Image.new("RGBA", (20, 20), (0x14, 0x0e, 0x1a, 255))
            ImageDraw.Draw(well).rectangle((1, 1, 18, 18), fill=(0x6e, 0x1a, 0x4c, 255))
            well.alpha_composite(im[0], (1, 1))
            for k in (1, 6):
                p = d / f"emblem_in_well_x{k}.png"
                well.resize((20 * k, 20 * k), Image.NEAREST).save(p)
                written.append(p)
    lines = sorted(f"{sha256_file(p)}  {p.relative_to(OUT).as_posix()}" for p in written if p.is_file())
    (OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines)} evidence files under {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
