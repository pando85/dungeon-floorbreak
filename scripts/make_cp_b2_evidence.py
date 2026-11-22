#!/usr/bin/env python3
"""Assemble the CARL-PROG-01 CP-B2 and CP-C evidence from real window captures.

Input: the capture folders written by scripts/make_cp_b2_evidence.sh. Output (in
docs/evidence/carl-prog-01/cp-b2-c): one 1x GIF (the whole 640x360 frame) and one
3x GIF (a nearest-neighbour crop around the action) per sequence, the HUD socket
stills (locked, ready, live, recharging, grant notice) and SHA256SUMS. None of this
is a design approval: the throw and stick art is prototype_fixture_unapproved.
"""
from __future__ import annotations

import hashlib
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:  # the guard below is testable without Pillow
    Image = None

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs/evidence/carl-prog-01/cp-b2-c"
# The crop (left, top, right, bottom) of the 3x GIFs and the plate of the HUD stills.
ACTION = (200, 170, 520, 290)
PLATE = (0, 0, 170, 50)
# The boss arena frames the rack and the boss above and left of Carl.
CROPS = {"carl_prog_dyn_rack_strut_188": (0, 60, 320, 290)}
NAMES = {
    "carl_prog_dyn_arc_standing_4": "throw-standing",
    "carl_prog_dyn_arc_moving_4": "throw-moving",
    "carl_prog_dyn_chain_depth_cap_188": "chain",
    "carl_prog_dyn_breaks_shore_188": "shore-collapse",
    "carl_prog_dyn_wall_slam_188": "wall-slam",
    "carl_prog_dyn_bowling_188": "bowling",
    "carl_prog_dyn_rack_strut_188": "rack-strut",
}
STILLS = {
    "carl_prog_special_locked_20": "hud-socket-locked",
    "carl_prog_dyn_arc_standing_3": "hud-socket-ready",
    "carl_prog_dyn_arc_standing_100": "hud-socket-live",
    "carl_prog_dyn_arc_standing_230": "hud-socket-recharging",
}


MARKER = ".capture-dir"
GRANT_KEY = "carl_prog_story_event_grants_dynamite_70"


def check_captures(caps: Path) -> list[Path]:
    """Validate the capture directory before anything is read or written (CP-B2-FU-EVID-RM).

    It must resolve to a directory strictly below <repo>/artifacts that holds the marker
    scripts/make_cp_b2_evidence.sh writes, and every folder in it must be a sequence this
    script knows, with frames. Returns the folders. Raises SystemExit otherwise, with the
    evidence directory untouched.
    """
    resolved = Path(caps).resolve()
    allowed = (ROOT / "artifacts").resolve()
    if allowed not in resolved.parents:
        raise SystemExit(f"refusing captures directory '{caps}': it must be a directory below {allowed.name}/ of this repository")
    if not resolved.is_dir() or not (resolved / MARKER).is_file():
        raise SystemExit(f"refusing captures directory '{caps}': no {MARKER} marker (it is not a capture directory made by scripts/make_cp_b2_evidence.sh)")
    folders = sorted(p for p in resolved.iterdir() if p.is_dir())
    if not folders:
        raise SystemExit(f"captures directory '{caps}' holds no capture folder")
    known = set(NAMES) | set(STILLS) | {GRANT_KEY}
    for folder in folders:
        key = folder.name.split("_", 1)[-1]
        if key not in known:
            raise SystemExit(f"captures directory '{caps}': unknown capture folder '{folder.name}'")
        if not any(folder.glob("frame_*.png")):
            raise SystemExit(f"captures directory '{caps}': '{folder.name}' has no frame_*.png")
    return folders


def frames(folder: Path) -> list[Image.Image]:
    return [Image.open(p).convert("RGB") for p in sorted(folder.glob("frame_*.png"))]


def gif(images: list[Image.Image], path: Path, ms: int) -> None:
    pal = [im.quantize(colors=128, method=Image.Quantize.MEDIANCUT) for im in images]
    pal[0].save(path, save_all=True, append_images=pal[1:], duration=ms, loop=0,
                optimize=True, disposal=1)


def main(caps: Path) -> None:
    folders = check_captures(caps)
    if Image is None:
        raise SystemExit("Pillow is required")
    OUT.mkdir(parents=True, exist_ok=True)
    for folder in folders:
        key = folder.name.split("_", 1)[1]
        images = frames(folder)
        if key in NAMES:
            name = NAMES[key]
            gif(images, OUT / f"{name}-1x.gif", 100)
            box = CROPS.get(key, ACTION)
            crop = [im.crop(box).resize(((box[2] - box[0]) * 3, (box[3] - box[1]) * 3),
                                        Image.NEAREST)
                    for im in images]
            gif(crop, OUT / f"{name}-3x.gif", 100)
        elif key == GRANT_KEY:
            # The whole frame: the plate (socket equipped) and the System notice.
            images[0].save(OUT / "hud-grant-notice.png", optimize=True)
        elif key in STILLS:
            im = images[0].crop(PLATE)
            im.resize((im.width * 4, im.height * 4), Image.NEAREST).save(
                OUT / f"{STILLS[key]}.png", optimize=True)
    sums = []
    for path in sorted(OUT.iterdir()):
        if path.suffix in (".gif", ".png"):
            sums.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1] if len(sys.argv) > 1 else "artifacts/cp-b2-captures"))
