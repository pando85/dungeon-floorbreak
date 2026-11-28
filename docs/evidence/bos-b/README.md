# BOS-B evidence

Evidence for [the BOS-B mini-spec](../../01-design/floor-01-bos-b-tusklings.md): the Ball of Swine's Tusklings in room 18 (`--scene f01_late --f01-room bos`), captured in the real Bevy window (xvfb, `--scale 1`, 640 x 360). The TUSK-01 frames are prototype art (`prototype_fixture_unapproved`, owner review pending), drawn native and never scaled. Nothing here is accepted; game feel is the owner's call.

Every still is captured **twice** (the two PNGs are byte-identical, `passes_byte_identical=yes`), and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`, 4 of 4): the art is presentation only. Regenerate with `bash scripts/make_bosb_evidence.sh` (needs the debug binary, `scripts/export_assets.sh`, xvfb, flock and ImageMagick).

## Stills (`scene_*_640x360.png`)

`bos_kick` and `bos_env` are evidence-only room pilots (`src/ball_fixtures.rs`): `bos_kick` is the bait pilot that, once the first crash has shed its Tusklings, stands in the arch pocket and kicks the one that comes in reach; `bos_env` is `carl_environment` (the weight and the jab). Captions claim only what is in frame.

| File | Pilot, tick | What is in frame |
| --- | --- | --- |
| `scene_crash_*` | `bos_kick`, 136 | The first crash at the left arch (the ball in its stun): two Tusklings just detached on its right flank, one in a tux and one in a gown |
| `scene_tell_*` | `bos_kick`, 238 | A Tuskling in its tell hold (head low, tusks level), the second still walking |
| `scene_kick_*` | `bos_kick`, 291 | The kicked Tuskling in flight over the ball's flank, the tick after the contact that staggered the ball for 0 HP |
| `scene_cower_*` | `bos_env`, 775 | The ball down and the exit door open; one Tuskling cowering in front of the landed weight (eyes shut, ears flat) |

## Other files

| File | Content |
| --- | --- |
| `capture_lines.txt` | one `BOSB_STILL` line per still: pilot, tick, the window and headless hashes, `hash_match`, `passes_byte_identical` and the PNG's SHA-256 |
| `SHA256SUMS` | the checksums of every file here (but this README) |
