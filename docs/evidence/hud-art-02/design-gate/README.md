# HUD-ART-02 design gate (keys at 1x)

Design-stage readability gate for the [HUD-ART-02 spec](../../../02-art/hud-art-02-spec.md), section 7. These are **mock composites**, not game captures of the new HUD: the room frames are real 1x window captures of this branch's build, and the design keys were pasted onto them at 1x. The executor's evidence replaces them with real captures.

How they were made:

- Room frames: `floor01_scene --scale 1 --no-audio --scene f01_late --f01-room ROOM --f01-loadout all --room-pilot PLAN --capture-sequence 240:1:1` (plus `--deck` for 640x400), under xvfb. Mean world luminance picked the darkest room (`hazards`) and the brightest (`dozer`).
- Carl's plate: the pad glyphs replaced by `key_j`, `key_k`, `key_l` (`key_k` and `key_l` built from the spec's symbol table).
- Donut's plate: her human plate from a `hoarder` `direct:2p` capture, with the 72 px mana track and `BLINK` (Silkscreen Regular 8 px, `amber`, right edge +157, text y +28).
- Notice: the new plate key with the item well and chip; the headline and `SYSTEM NOTICE` pixels are the game's own 1x renders, lifted from `docs/evidence/f01-playtest` stills.

| File | Shows |
| --- | --- |
| `keys-sheet-1x.png`, `keys-sheet-8x.png` | the existing A/B/X/Y disks, then `key_j`, `key_arrow_right`, `chip_dynamite`, `chip_puddle` on `plate_fill` |
| `carl-plate-keyboard-4x.png` | Carl's plate with keyboard caps |
| `donut-plate-blink-4x.png` | Donut's human plate, short track, `BLINK` above the socket |
| `hazards-360-puddle-before-notice-3x.png`, `hazards-360-puddle-after-notice-3x.png` | the current notice frame and the new plate with the puddle well, darkest room |
| `dozer-400-dynamite-before-notice-3x.png`, `dozer-400-dynamite-after-notice-3x.png` | the same with the dynamite well, brightest room, 640x400 |
| `dozer-400-puddle-after-notice-3x.png` | the puddle well in the brightest room, next to the System face (AT-10) |
| `hazards-360-puddle-after-1x.png`, `hazards-400-puddle-after-1x.png`, `dozer-360-dynamite-after-1x.png`, `dozer-400-dynamite-after-1x.png` | full frames at 1x |

Notice edge contrast against the median background just outside the notice: new `metal_hi` rim 4.7:1 (`hazards`) and 4.6 to 4.7:1 (`dozer`); the current outer `hud_ink` row 1.1:1 and `magenta` ring 4.2 to 4.4:1.

Rights: original HUD art under the recorded `ART-PUBLICATION-ALL-V1` scope; no concept PNG was sampled or traced. Checksums: [`SHA256SUMS`](SHA256SUMS).
