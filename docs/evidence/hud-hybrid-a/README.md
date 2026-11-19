# Evidence: HUD-HYBRID-A-01

Evidence for PR #103 (HUD-HYBRID-A-01). Checksums: [SHA256SUMS](SHA256SUMS). Spec: [hybrid-hud-a-native-spec.md](../../02-art/hybrid-hud-a-native-spec.md). Art atlas evidence: [hud-hybrid-a-01](../hud-hybrid-a-01/).

## Real Bevy captures

All `hud-*` images are real window captures of `floor01_scene` with the **real exported atlas** (`generated/art/hud_hybrid_a.png`, no placeholders), made with `scripts/ci_floor01_render_smoke.sh hud` and `hud-deck` (12 states each, all passed `scripts/check_hud_capture.py`), then reduced or tiled by integer nearest-neighbour only. Scene: `walk_right` scenario, frozen at tick 250, lit (A1). Captured on a local machine (RTX 4060 Ti under Xvfb), not a Deck.

* **REAL** in the filename: live game state. Carl and Donut HP come from the simulation, the `COMPANION` label, `FLOOR 01`, no mana, no boss. `normal` is the story corridor: melee is ignored there, so Carl's row shows **no PUNCH/KICK chips**, only the locked socket (spec 2.4, review R1). The other REAL states are combat fixtures (`cp01_goblin_attacks`, `cp01_retry`, `cp01_kick`, `cp03_shore_kill`) where the chips are live, and they carry real events: a real hurt flash, Carl at 1 HP (cracked heart), and the real `LAUNCHED!` and `FLATTENED!` notices.
* **PREVIEW** in the filename: a `--hud-preview <name>` fixture. The frame carries the `PREVIEW <NAME>` tag. These states are not implemented in gameplay; see the HUD fixture debt in the backlog.

| File pattern | What |
| --- | --- |
| `hud-REAL-normal-<size>-1x.png` | normal play |
| `hud-PREVIEW-{pre-unlock,post-unlock,depleted,boss,coop}-<size>-1x.png` | the five fixtures |
| `hud-REAL-{hurt,low-hp,notice-launched,notice-flattened}-<size>-1x.png` | real combat states: hurt flash (`cp01_goblin_attacks` tick 250), Carl at 1 HP with the cracked heart (`cp01_retry` tick 340), `LAUNCHED!` (`cp01_kick` tick 300), `FLATTENED!` (`cp03_shore_kill` tick 170) |
| `hud-PREVIEW-post-unlock-dualsense-<size>-1x.png` | `--glyphs=dualsense` (Square and Triangle) on the post-unlock fixture |
| `hud-PREVIEW-depleted-reduced-motion-<size>-1x.png` | `--hud-reduced-motion` on the depleted fixture (hatched flash, no blink or slide) |
| `hud-contact-sheet-<size>-2x.png` | normal plus the five fixtures, 2x captures tiled, REAL / PREVIEW label above each |
| `hud-contact-sheet-real-and-options-<size>-2x.png` | the four real combat states, DualSense and reduced motion, 2x captures tiled |
| `hud-REAL-normal-640x360-3x-1920x1080.png` | one 1920x1080 frame, normal play (primary target, scale 3) |
| `hud-PREVIEW-depleted-640x360-3x-1920x1080.png` | one 1920x1080 frame, combat `depleted` fixture |

`<size>` is `640x360` (default view) or `640x400` (`--deck`). `1x` files are the virtual canvas size (the checker proves every 2x2 block is flat, so the reduction loses nothing).

## Gameplay clearance (spec 8.2)

Measured by `cargo test --lib gameplay_stays_below_the_hud_plates -- --nocapture` over all 13 combat fixtures and the `walk_right`, `turn_around` and `run_jump` corridor scenarios (hero and enemy bodies, launched goblins, swing boxes; screen y = layout y minus the view top). Required: top at y >= 60.

| Canvas | Minimum top y | Where | Fixtures only |
| --- | --- | --- | --- |
| 640x360 | 147.5 | hero jump apex off the raised floor | 225.1 (`cp01_retry`) |
| 640x400 | 167.5 | same | 245.1 (`cp01_retry`) |

Result: **passes**; the closest gameplay approaches the plates' bottom (y = 56) from 91 px away or more. Not measured: cosmetic death parts and camera shake (not simulation state).

## Known deviations

* Fixture ages are static, so the `depleted` notice and blinks are frozen mid-state.
* The muted-chip and boss-warning look use a code tint of an existing atlas frame.
* The CRT screen flash is not drawn.
* The Donut portrait eyes (rows 12 to 17) follow the approved art as exported.
* Notice copy is placeholder pending an owner tone check.

## Design diagrams

`zones_640x360_2x.png` and `zones_640x400_2x.png` are the design-step layout zone diagrams (green action lane and walk line, yellow top rects, cyan bottom rects, magenta 8 px safe margin), at 2x. They are **design diagrams, not Bevy captures**.
