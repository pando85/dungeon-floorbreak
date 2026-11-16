# Floor 1 — Combat-first linear greybox (WORK-021)

**Status:** implementation prototype; requires CI green and human gameplay review.
**Engine:** Rust 2024 + Bevy 0.20.1 (final decision).
**Scope:** IP-neutral placeholder geometry. Approved Carl and Donut designs are not redrawn.

## Player-facing direction

Move primarily **right**, through short combat encounters. Left movement is retained for tactical repositioning, but there are no maze branches. Bombs destroy timber and enemies; steel and masonry require the player to climb platforms and continue right. The floor keeps the approved monumental-catacomb + rusty-industrial A+B visual direction without foregrounding televised cruelty.

## Authored prototype geometry (virtual pixels)

- 0–610: monumental entrance and first goblin encounters (hero at x=86).
- 610–644: destructible timber obstacle.
- 644–1100: more ground encounters.
- 1100–1222: first elevated catwalk, top y=-40.
- 1220–1326: second catwalk, top y=26.
- 1340–1388: reinforced steel wall, top y=60, impossible to bomb; climb over it.
- 1400–1515: descent catwalk.
- 1610–1644: second timber obstacle.
- 1976: finish trigger.

The world is 2048 pixels wide, extending over multiple 640×360 camera screens. Layout numbers are initial gameplay hypotheses, not final level geometry.

## Controls

| Action | Keyboard | First controller |
| --- | --- | --- |
| Move | A/D or arrows | Left stick/D-pad |
| Jump | Space | South |
| Melee | J | West |
| Timed bomb | K | East |

Two-hit goblin ground chasers, four player hit points, temporary invulnerability and instant retry keep the experiment focused. There is no approved enemy animation, Donut implementation, sound or HUD yet.

## Implementation and verification

- src/prototype.rs: pure fixed-step (60 Hz) simulation; gravity/collisions, buffered jump/coyote time, melee, bombs, destructible material, AI.
- src/main.rs: Bevy gamepad/keyboard input adapter, placeholder sprites, scroll camera and visibility updates.
- Test with cargo fmt --all -- --check, cargo clippy --locked --all-targets -- -D warnings, cargo test --locked --all-targets.
- Run the headless scenario with cargo run -- --headless-combat-smoke.
- Use the Linux software Vulkan/Xvfb CI screenshot as technical evidence, NOT final pixel art approval.
- Manually play with a controller and review combat tempo, routes, jump feel, bomb timing and fast restarts before accepting.

## Follow-ups (outside this PR)

1. Adjust mechanics with playtest evidence; add small hitstop and enemy tells.
2. Prototype Donut's spell and solo switching/co-op.
3. Integrate approved early Carl 52 px and Donut 40 px only after editable native sprite masters and atlas QA.
4. Move geometry to authored LDtk while retaining pure Rust gameplay rules.
5. Author Aseprite-native environment kit from the approved A+B mood references, not flattened concept-art crops.

Rights: a public IP-neutral engine prototype is not permission to distribute a franchise-based game.
