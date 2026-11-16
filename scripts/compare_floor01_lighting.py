#!/usr/bin/env python3
"""Compare a Bevy lit capture with the ENV-01 Pillow lit preview (COMP-02 evidence).

Both frames show the same camera window and actor pose (layout view ``abc_640x360``,
Carl and Donut at the preview's ``actor_check`` positions, torch time 0). The Pillow
frame (``scripts/env_kit_light.py``) is the owner-approved A1 reference model; the
Bevy frame is the real-time lighting pass. This script reports how far apart they are
and writes a labelled side-by-side. It makes no art judgement: it measures.

Region masks come from the kit itself (rendered layers cropped to the view):

* ``wall``    backwall pixels not covered by decals, structure, props, collision or actors;
* ``gallery`` parallax pixels seen through the arch (nothing nearer covers them);
* ``floor``   the deck tiles of the collision layer, minus actors;
* ``actors``  opaque pixels of Carl and Donut.

Numbers: mean CIE L* of each frame per region, delta E76 per pixel (mean and 95th
percentile), whole-frame mean L*, and the share of warm (R > B + 8) and cold (B > R + 8)
pixels. Needs Pillow and numpy.

Usage::

    python3 scripts/compare_floor01_lighting.py --bevy bevy_1920x1080.png \
        --pillow docs/evidence/floor01-catacomb-kit/lit_abc_1920x1080_x3.png \
        --out-image side_by_side.png --out-json lighting_compare.json
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import env_kit as ek  # noqa: E402

LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
WARM_COLD_MARGIN = 8
# Divergence gates. Measured Bevy-vs-Pillow on a real GPU is mean dE76 ~0.55,
# p95 ~0.9, |dL*| ~0.2; the gates leave a wide margin for software rasterisers
# and still fail on a real regression (a missing pass is dE76 > 10, a wrong
# ambient or haze moves the means by several units).
GATE_MEAN_DE = 1.5
GATE_P95_DE = 3.0
GATE_ABS_DL = 1.0
GATE_REGION_DE = 2.5


def srgb_to_lab(rgb: np.ndarray) -> np.ndarray:
    """sRGB uint8 (..., 3) to CIE L*a*b* (D65)."""
    c = rgb.astype(np.float64) / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    m = np.array([[0.4124564, 0.3575761, 0.1804375],
                  [0.2126729, 0.7151522, 0.0721750],
                  [0.0193339, 0.1191920, 0.9503041]])
    xyz = lin @ m.T / np.array([0.95047, 1.0, 1.08883])
    f = np.where(xyz > 216 / 24389, np.cbrt(xyz), (24389 / 27 * xyz + 16) / 116)
    return np.stack([116 * f[..., 1] - 16, 500 * (f[..., 0] - f[..., 1]),
                     200 * (f[..., 1] - f[..., 2])], axis=-1)


def region_masks(view: list[int]) -> dict[str, np.ndarray]:
    """Boolean masks at the virtual (640x360) view size."""
    layout = ek.load_json(LAYOUT)
    kit = ek.load_kit(layout["kit_contract"])
    vx, vy, vw, vh = view
    layers = ek.render_layers(kit, layout, time_ms=0, camera_x=vx)
    pair = layout["preview"]["actor_check"][:2]
    actors, _boxes = ek.actor_layer(kit, layout, pair)

    def alpha(img: Image.Image) -> np.ndarray:
        return np.asarray(img.crop((vx, vy, vx + vw, vy + vh)).getchannel("A")) > 0

    a = {name: alpha(img) for name, img in layers.items()}
    act = alpha(actors)
    floor = np.zeros((vh, vw), dtype=bool)
    for r, row in enumerate(layout["tile_layers"]["collision"]):
        for x, tid in enumerate(row):
            if tid and tid.startswith("deck"):
                y0, y1 = r * 16 - vy, r * 16 + 16 - vy
                x0, x1 = x * 16 - vx, x * 16 + 16 - vx
                floor[max(0, y0):max(0, y1), max(0, x0):max(0, x1)] = True
    near = (a["backwall_decal"] | a["collision"] | a["structure"] | a["prop"] | a["foreground"] | act)
    return {
        "wall": a["backwall"] & ~near,
        "gallery": (a["parallax_far"] | a["parallax_mid"]) & ~a["backwall"] & ~near,
        "floor": floor & ~act & ~a["prop"] & ~a["foreground"],
        "actors": act,
    }


def upscale(mask: np.ndarray, k: int) -> np.ndarray:
    return np.kron(mask, np.ones((k, k), dtype=bool))


def share(rgb: np.ndarray, sel: np.ndarray | None = None) -> tuple[float, float]:
    r = rgb[..., 0].astype(int)
    b = rgb[..., 2].astype(int)
    warm = r - b > WARM_COLD_MARGIN
    cold = b - r > WARM_COLD_MARGIN
    if sel is not None:
        warm, cold = warm[sel], cold[sel]
    return float(warm.mean()), float(cold.mean())


def compare(bevy: np.ndarray, ref: np.ndarray, masks: dict[str, np.ndarray]) -> dict:
    lab_b, lab_r = srgb_to_lab(bevy), srgb_to_lab(ref)
    de = np.linalg.norm(lab_b - lab_r, axis=-1)
    out: dict = {"regions": {}}
    for name, mask in masks.items():
        n = int(mask.sum())
        if n == 0:
            out["regions"][name] = {"pixels": 0}
            continue
        out["regions"][name] = {
            "pixels": n,
            "mean_L_bevy": round(float(lab_b[..., 0][mask].mean()), 2),
            "mean_L_pillow": round(float(lab_r[..., 0][mask].mean()), 2),
            "delta_L": round(float(lab_b[..., 0][mask].mean() - lab_r[..., 0][mask].mean()), 2),
            "mean_dE76": round(float(de[mask].mean()), 2),
            "p95_dE76": round(float(np.percentile(de[mask], 95)), 2),
            "mean_abs_rgb": round(float(np.abs(bevy.astype(int) - ref.astype(int))[mask].mean()), 2),
        }
    wb, cb = share(bevy)
    wr, cr = share(ref)
    out["whole_frame"] = {
        "mean_L_bevy": round(float(lab_b[..., 0].mean()), 2),
        "mean_L_pillow": round(float(lab_r[..., 0].mean()), 2),
        "delta_L": round(float(lab_b[..., 0].mean() - lab_r[..., 0].mean()), 2),
        "mean_dE76": round(float(de.mean()), 2),
        "p95_dE76": round(float(np.percentile(de, 95)), 2),
        "max_dE76": round(float(de.max()), 2),
        "pixels_dE76_over_3": int((de > 3).sum()),
        "warm_share_bevy": round(wb, 4), "warm_share_pillow": round(wr, 4),
        "cold_share_bevy": round(cb, 4), "cold_share_pillow": round(cr, 4),
        "warm_cold_margin_8bit": WARM_COLD_MARGIN,
    }
    out["_de"] = de
    return out


def check_gates(result: dict, args) -> list[str]:
    whole = result["whole_frame"]
    failures = []
    if whole["mean_dE76"] > args.max_mean_de:
        failures.append(f"whole-frame mean dE76 {whole['mean_dE76']} > {args.max_mean_de}")
    if whole["p95_dE76"] > args.max_p95_de:
        failures.append(f"whole-frame p95 dE76 {whole['p95_dE76']} > {args.max_p95_de}")
    if abs(whole["delta_L"]) > args.max_abs_dl:
        failures.append(f"whole-frame |delta L*| {abs(whole['delta_L'])} > {args.max_abs_dl}")
    for name, region in result["regions"].items():
        if region.get("pixels", 0) == 0:
            failures.append(f"region {name} has no pixels (mask broken)")
        elif region["mean_dE76"] > args.max_region_mean_de:
            failures.append(f"region {name} mean dE76 {region['mean_dE76']} > {args.max_region_mean_de}")
    return failures


def label(img: Image.Image, text: str) -> Image.Image:
    band = 30
    out = Image.new("RGB", (img.width, img.height + band), (16, 14, 20))
    out.paste(img, (0, band))
    ImageDraw.Draw(out).text((8, 8), text, fill=(236, 224, 200))
    return out


def heat(de: np.ndarray, cap: float = 20.0) -> Image.Image:
    t = np.clip(de / cap, 0, 1)
    rgb = np.stack([np.clip(t * 2, 0, 1), np.clip(t * 2 - 0.6, 0, 1), np.clip(0.25 + t * 0.2, 0, 1)], -1)
    return Image.fromarray((rgb * 255).astype(np.uint8))


def side_by_side(bevy_img: Image.Image, ref_img: Image.Image, de: np.ndarray, size: tuple[int, int]) -> Image.Image:
    def fit(img):
        return img.resize(size, Image.Resampling.BOX)

    panels = [
        label(fit(bevy_img), "BEVY lighting pass (floor01_scene --reference-frame), box-downscaled"),
        label(fit(ref_img), "PILLOW reference (env_kit_light.py, ENV-01 lit preview), box-downscaled"),
        label(fit(heat(de)), "delta E76 per pixel (Bevy vs Pillow), black 0 .. bright 20+"),
    ]
    gap = 12
    sheet = Image.new("RGB", (sum(p.width for p in panels) + gap * 2, panels[0].height), (16, 14, 20))
    x = 0
    for p in panels:
        sheet.paste(p, (x, 0))
        x += p.width + gap
    return sheet


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--bevy", type=Path, required=True)
    parser.add_argument("--pillow", type=Path, required=True)
    parser.add_argument("--out-image", type=Path, required=True)
    parser.add_argument("--out-json", type=Path, required=True)
    parser.add_argument("--view", default="abc_640x360")
    parser.add_argument("--max-mean-de", type=float, default=GATE_MEAN_DE,
                        help="gate: whole-frame mean dE76 (default %(default)s)")
    parser.add_argument("--max-p95-de", type=float, default=GATE_P95_DE,
                        help="gate: whole-frame 95th percentile dE76 (default %(default)s)")
    parser.add_argument("--max-abs-dl", type=float, default=GATE_ABS_DL,
                        help="gate: whole-frame |mean L* difference| (default %(default)s)")
    parser.add_argument("--max-region-mean-de", type=float, default=GATE_REGION_DE,
                        help="gate: mean dE76 of every region (default %(default)s)")
    parser.add_argument("--no-gate", action="store_true", help="report only, never fail")
    args = parser.parse_args(argv)
    layout = ek.load_json(LAYOUT)
    view = layout["views"][args.view]
    with Image.open(args.bevy) as im:
        bevy_img = im.convert("RGB")
    with Image.open(args.pillow) as im:
        ref_img = im.convert("RGB")
    if bevy_img.size != ref_img.size:
        print(f"size mismatch: bevy {bevy_img.size}, pillow {ref_img.size}", file=sys.stderr)
        return 1
    k = bevy_img.width // view[2]
    if bevy_img.width != view[2] * k or bevy_img.height != view[3] * k:
        print(f"image size {bevy_img.size} is not an integer multiple of the view", file=sys.stderr)
        return 1
    masks = {name: upscale(m, k) for name, m in region_masks(view).items()}
    result = compare(np.asarray(bevy_img), np.asarray(ref_img), masks)
    de = result.pop("_de")
    result["format"] = "floor01-lighting-compare-v1"
    result["view"] = args.view
    result["scale"] = k
    result["note"] = ("Bevy real-time pass vs the Pillow reference model; a measurement, not an "
                      "art-acceptance claim. dE76 = Euclidean distance in CIE L*a*b* (D65).")
    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    args.out_image.parent.mkdir(parents=True, exist_ok=True)
    side_by_side(bevy_img, ref_img, de, (bevy_img.width // 2, bevy_img.height // 2)).save(args.out_image)
    print(json.dumps(result, indent=2))
    failures = check_gates(result, args)
    result["gates"] = {"max_mean_dE76": args.max_mean_de, "max_p95_dE76": args.max_p95_de,
                       "max_abs_delta_L": args.max_abs_dl, "max_region_mean_dE76": args.max_region_mean_de,
                       "enforced": not args.no_gate, "failures": failures}
    args.out_json.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for f in failures:
        print(f"GATE FAILED: {f}", file=sys.stderr)
    if failures and not args.no_gate:
        return 2
    print("COMPARE_GATES_OK" if not failures else "COMPARE_GATES_FAILED (not enforced)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
