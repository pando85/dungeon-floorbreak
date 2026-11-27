# BOS-ART evidence

Evidence for [the BOS-ART spec](../../02-art/bos-art-spec.md): the Ball of Swine's room 18 (`--scene f01_late --f01-room bos`) dressed and drawn from the SWINE-01 atlas, captured in the real Bevy window (xvfb, `--scale 1`, 640 x 360). **Owner art review pending**; nothing here is accepted, and game feel is the owner's call.

Every still is captured **twice** (the two PNGs are byte-identical, `passes_byte_identical=yes`), and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`, 7 of 7): the art is presentation only. Regenerate with `bash scripts/make_bosart_evidence.sh` (needs the debug binary, `scripts/export_assets.sh`, xvfb, flock, ImageMagick and Pillow).

## Stills (`scene_*_640x360.png`)

The `bos` pilot is the BOS-A bait-and-punish Carl; `bos_weight` is the evidence-only pilot of BOS-ART-AD11 (two post hits from the left alcove, a third on the crash). Captions claim only what is in frame.

| File | Pilot, tick | What is in frame |
| --- | --- | --- |
| `scene_idle_*` | `bos`, 20 | The ball standing right of centre on the idle frames, the weight hung from the girder under the top edge with its tie chain to the timber post, the two arches with torches on their lintels, the copper run and valve at the left, Carl and Donut at the left |
| `scene_tell_*` | `bos`, 55 | The roll tell: the coiled frame (rocked back, eyes wide), the arch ahead |
| `scene_roll_*` | `bos`, 100 | Mid-roll on one of the six native roll frames, running at the left arch |
| `scene_crash_*` | `bos`, 135 | The crash at the left arch: the impact frame against the arch, the heroes at its foot |
| `scene_stun_*` | `bos`, 150 | The stun: the open-mouthed rebound frame, the weight still hung |
| `scene_pinned_*` | `bos_weight`, 200 | The weight landed on the pinned ball: the slab on the squashed heap, the chain heaped on it, the girder bare |
| `scene_exit_open_*` | `bos`, 1900 | The ball down (the heap), the exit door open at the right with its lit interior (the stairwell), the heroes walking to it |

## Other files

| File | Content |
| --- | --- |
| `capture_lines.txt` | one `BOS_ART_STILL` line per still: pilot, tick, the window and headless hashes, `hash_match`, `passes_byte_identical` and the PNG's SHA-256 |
| `contact_x4_weight.png` | the chain-weight key at x4 (hung, landed, the two chain tiles) and at 1x on a flat ground, unlit |
| `measurements.txt` | the key sizes, colours and pins (`KEY` lines, pinned by `tests/test_art_bosart.py`) and the placement boxes of the dressing (`DRESSING` lines) |
| `SHA256SUMS` | the checksums of every file here |
