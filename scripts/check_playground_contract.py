#!/usr/bin/env python3
"""Validate the runtime's deliberately pinned cast/atlas contract.

This complements validate_assets.py, which validates the full asset catalogue.
Do not confuse approved opaque height with the transparent canvas size.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

CAST = (
    ("carl_book1_h52", (36, 56), (19, 55), 52),
    ("donut_scale_h40", (46, 44), (22, 43), 40),
    ("mongo_adult_h76", (108, 80), (68, 79), 76),
)


def main() -> None:
    for name, size, pivot, opaque_height in CAST:
        manifest = json.loads((ROOT / "assets/manifests" / f"{name}.json").read_text())
        metadata = json.loads((ROOT / manifest["output_metadata"]).read_text())
        image = ROOT / manifest["output_atlas"]
        assert image.is_file(), f"{name}: exported atlas missing"
        assert tuple(manifest["frame_size"]) == size, f"{name}: canvas changed"
        assert tuple(manifest["pivot"]) == pivot, f"{name}: foot pivot changed"
        assert name.endswith(f"h{opaque_height}"), f"{name}: opaque scale changed"
        assert manifest["animation_tags"] == ["idle"], f"{name}: unexpected animation tags"
        frames = metadata["frames"]
        assert len(frames) == 4, f"{name}: expected four idle frames"
        assert metadata["meta"]["frameTags"][0]["name"] == "idle"
        assert metadata["meta"]["frameTags"][0]["from"] == 0
        assert metadata["meta"]["frameTags"][0]["to"] == 3
        for index, frame in enumerate(frames):
            rect = frame["frame"]
            assert (rect["x"], rect["y"], rect["w"], rect["h"]) == (
                index * size[0], 0, size[0], size[1]
            ), f"{name}: Bevy expects unpadded row-major frames"
        print(f"PLAYGROUND_CONTRACT_OK: {name} {opaque_height}px, 4 frames, pivot {pivot}")


if __name__ == "__main__":
    main()
