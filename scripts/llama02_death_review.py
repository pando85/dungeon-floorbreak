"""LLAMA-02-DEATH review-pack hook (named by ``review.hook`` in ``llama_death_h64.pixelart.json``).

The crush frames carry ``hazard_draw: "over"`` in ``assets/manifests/llama_death_h64.json``: on those ticks the falling
stone draws over the crushed llama, anchored on the frame's ``hit_origin`` (spec section 13.1). Without it the crush
reads as a llama lying down next to a lava jet, so the review images draw it. From ``crushed_remains`` on the goblin
order applies: the landed stone sits behind the remains. The hook is opt-in and asset-local: the shared toolkit only
calls it when the config names it.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

_CACHE: dict = {}


def _data() -> dict:
    if not _CACHE:
        import author_llama02_death as ad
        manifest = json.loads((ROOT / "assets/manifests/llama_death_h64.json").read_text(encoding="utf-8"))
        frames = manifest["death_poses"]["frames"]
        _CACHE["over"] = {f["index"]: f["anchors"]["hit_origin"] for f in frames if f.get("hazard_draw") == "over"}
        _CACHE["remains"] = {f["index"]: f["anchors"]["pool_origin"] for f in frames if f["tag"] == "crushed_remains"}
        _CACHE["falling"] = ad.block_piece("loose_block_falling")
        _CACHE["landed"] = ad.block_piece("loose_block_landed")
        _CACHE["canvas"], _CACHE["pivot"] = ad.CANVAS, ad.PIVOT
    return _CACHE


def actor_overlays(cfg, actor: dict):
    """``(before, after)`` scene entries around a mirrored actor at world ``x`` (the runtime flip: x' = w - 1 - x, pivot
    column 51)."""
    d = _data()
    frame = actor["frame"]
    w, pivot = d["canvas"][0], d["pivot"]
    flipped_pivot = w - 1 - pivot[0]
    if frame in d["over"]:
        ax, ay = d["over"][frame]
        img, anchor = d["falling"]
        return [], [(img, anchor, actor["x"] + (w - 1 - ax) - flipped_pivot, pivot[1] - ay, False)]
    if frame in d["remains"]:
        ax, _ay = d["over"][0]
        img, anchor = d["landed"]
        return [(img, anchor, actor["x"] + (w - 1 - ax) - flipped_pivot, 0, False)], []
    return [], []


def strip_frame(cfg, frame: int, img):
    """The frame as shown in the strip (authored facing right): the stone over it on the hazard frames, the landed stone
    behind the remains."""
    d = _data()
    from PIL import Image
    out = Image.new("RGBA", img.size, (0, 0, 0, 0))

    def put(piece, ax, ay):
        stone, (sx, sy) = piece
        _clipped(out, stone, ax - sx, ay - sy)

    if frame in d["over"]:
        out.alpha_composite(img)
        ax, ay = d["over"][frame]
        put(d["falling"], ax, ay)
    elif frame in d["remains"]:
        ax, ay = d["over"][0]
        put(d["landed"], ax, d["pivot"][1])
        out.alpha_composite(img)
    else:
        return img
    return out


def _clipped(out, stone, x: int, y: int) -> None:
    """alpha_composite of a sprite that overhangs the frame (the overhang is cropped)."""
    cx, cy = max(0, -x), max(0, -y)
    w, h = min(stone.width, out.width - x), min(stone.height, out.height - y)
    out.alpha_composite(stone.crop((cx, cy, w, h)), (x + cx, y + cy))
