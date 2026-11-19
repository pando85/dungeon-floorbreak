# Evidence: HUD-HYBRID-A-01

Evidence for PR #103 (HUD-HYBRID-A-01). Checksums: [SHA256SUMS](SHA256SUMS). Spec: [hybrid-hud-a-native-spec.md](../../02-art/hybrid-hud-a-native-spec.md). Art atlas evidence: [hud-hybrid-a-01](../hud-hybrid-a-01/).

## Real Bevy captures

All `hud-*` images are real window captures of `floor01_scene` with the **real exported atlas** (`generated/art/hud_hybrid_a.png`, no placeholders), made with `scripts/ci_floor01_render_smoke.sh hud` and `hud-deck` (11 states each, all passed `scripts/check_hud_capture.py`), then reduced or tiled by integer nearest-neighbour only. Scene: `walk_right` scenario, frozen at tick 250, lit (A1). Captured on a local machine (RTX 4060 Ti under Xvfb), not a Deck.

* **REAL** in the filename: live game state. Carl and Donut HP come from the simulation, the `COMPANION` label, `FLOOR 01`, no mana, no boss, no notice.
* **PREVIEW** in the filename: a `--hud-preview <name>` fixture. The frame carries the `PREVIEW <NAME>` tag. These states are not implemented in gameplay; see the HUD fixture debt in the backlog.

| File pattern | What |
| --- | --- |
| `hud-REAL-normal-<size>-1x.png` | normal play |
| `hud-PREVIEW-{pre-unlock,post-unlock,depleted,boss,coop}-<size>-1x.png` | the five fixtures |
| `hud-contact-sheet-<size>-2x.png` | normal plus the five fixtures, 2x captures tiled, REAL / PREVIEW label above each |
| `hud-REAL-normal-640x360-3x-1920x1080.png` | one 1920x1080 frame, normal play (primary target, scale 3) |
| `hud-PREVIEW-depleted-640x360-3x-1920x1080.png` | one 1920x1080 frame, combat `depleted` fixture |

`<size>` is `640x360` (default view) or `640x400` (`--deck`). `1x` files are the virtual canvas size (the checker proves every 2x2 block is flat, so the reduction loses nothing).

## Known deviations

* Fixture ages are static, so the `depleted` notice and blinks are frozen mid-state.
* The muted-chip and boss-warning look use a code tint of an existing atlas frame.
* The CRT screen flash is not drawn.
* The Donut portrait eyes (rows 12 to 17) follow the approved art as exported.
* Notice copy is placeholder pending an owner tone check.

## Design diagrams

`zones_640x360_2x.png` and `zones_640x400_2x.png` are the design-step layout zone diagrams (green action lane and walk line, yellow top rects, cyan bottom rects, magenta 8 px safe margin), at 2x. They are **design diagrams, not Bevy captures**.
