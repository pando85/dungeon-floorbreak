# ANIM-INTERACT-01 evidence: Carl and Donut interact sets

**Status: all 12 Carl and 8 Donut frames drawn (6 frozen keys untouched); independent review pending; not owner-approved; not Bevy captures (nothing is wired into a runtime).**

Review renders of the [design spec](../../02-art/interact-anim-01-spec.md). Everything is composed from the grid sources (`assets/source/characters/{carl/carl_h52,donut/donut_h40}_interact.grid.json`) and the tick tables of the toolkit configs. The grids are rewritten deterministically by `python3 scripts/author_interact01.py` (`--check` verifies them). Regenerate this folder with:

```sh
python3 scripts/author_interact01.py --evidence                      # scene composites, keys at 1x, timing diagram
python3 -m scripts.pixelart evidence assets/source/characters/carl/carl_h52_interact.pixelart.json --out docs/evidence/anim-interact-01/carl
python3 -m scripts.pixelart evidence assets/source/characters/donut/donut_h40_interact.pixelart.json --out docs/evidence/anim-interact-01/donut
# then delete the toolkit's generic tell_attack_strip.png, scene_*.png and per-folder SHA256SUMS (not meaningful for this set) and rewrite the root SHA256SUMS
```

| File | What it shows |
| --- | --- |
| `carl_set_floor1_640x360.png`, `carl_set_floor1_1920x1080.png` | the lit Floor 1 corridor at the 640 x 360 virtual view (gameplay scale, x1) and the exact 3x 1080p copy: idle Carl, then all 12 interact frames in atlas order (u0-u3, p0-p3, s0-s3), 48 px apart. Each verb has a grey placeholder prop at its stand-off distance (lever with the handle up until the grip and thrown from the haul, floor item that rides in the far hand from p2, crate that gives 2 px after the contact tick) |
| `donut_set_floor1_640x360.png`, `donut_set_floor1_1920x1080.png` | the same for Donut: idle Donut, then d0-d3 and n0-n3, 58 px apart, with a low switch (paw) and a small item (nudge, gives 2 px after the contact tick); idle Carl on the right for scale |
| `keys-scale.png`, `keys-scale-x4.png` | the six frozen keys at 1x with placeholder props, unchanged from the design phase, and a 4x copy |
| `timing_diagram.png` | frame layout, pivot, 60 Hz tick windows and contact ticks of both sets, drawn from the manifests (`assets/manifests/{carl_book1_h52,donut_h40}_interact.json`) |
| `carl/contact_sheet_x1.png`, `carl/contact_sheet_x4.png`, `donut/contact_sheet_x1.png`, `donut/contact_sheet_x4.png` | all frames at native size and at 4x (the enlarged frames): tag, tick count, frozen keys flagged KEY, contact row (cyan), pivot (yellow) |
| `carl/gif_*_x1.gif`, `carl/gif_*_x3.gif`, `donut/gif_*_x1.gif`, `donut/gif_*_x3.gif` | one GIF per verb tag at 1x and 3x, played at the 60 Hz tick durations on the flat review background with the walking line; the last frame rests before the loop restarts |

`python3 -m scripts.pixelart review-pack <config>` needs the Floor 1 kit atlases first: run `bash scripts/export_assets.sh` once in a fresh checkout (otherwise it stops with "the Floor 1 kit atlases are missing" after writing the contact sheet).

The props, switch, crate and item rectangles are neutral grey and brown placeholders for stand-off distance and contact only, not prop art. The environment kit is the approved, unchanged ENV-01 kit. `SHA256SUMS` lists the hash of every file here.
