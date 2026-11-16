# Animation system — frame-by-frame first, modular selectively

**Decision:** detailed pixel art with a **hybrid** animation strategy: key gameplay motion uses curated pixel frames; secondary character expression may use modular layers. Avoid simply rotating tiny body-part PNGs for all movement: that produces pixel shimmer and puppet-like deformations.

## Animation classes

| Class | Examples | Technique |
| --- | --- | --- |
| Primary locomotion | run, takeoff, fall, land | Hand/key-pose sprite sequences, 6–8 frames for run as a hypothesis |
| Combat | attack, bomb plant, cast, hurt | 3–8 frames with explicit anticipation/active/recovery markers |
| Idle personality | head tilt, blink, tail gesture | Modular face/tail or small additive sprites |
| Reaction | hit pause, recoil, knockback | Game code controls timing; sprite impact frames |
| Environment FX | smoke, explosions, magic | Layered animated sprites + controlled particles |

## Frame timeline schema

Source-of-truth metadata (JSON per animation tag) contains `tag`, `direction`, `frames[]` each with `duration_ms`, `frame_rect`, optional `pivot`, `origin`, `hitbox`, `active_frames`, and event markers such as `footstep`, `bomb_spawn`, `cast_fire`. **Hitboxes are tuned via gameplay data**, not inferred from occupied pixels for authoritative collisions.

## Runtime invariants

- Logic fixed-step, animation visual-step; frame times do not alter physical attack cooldowns.
- Hit frames are deterministic; events emitted once per state transition.
- Rigs and animations should preserve character pivot regardless of frame dimensions.
- Directional flip is acceptable for neutral attacks; asymmetrical gear/UI must be handled explicitly.
- Maintain 1-pixel camera/sprite alignment at presentation; physics retains continuous coordinates.
- Cap superfluous smear frames; the gameplay needs to remain readable during explosions.

## Character-specific cues

- Carl / demolitionist: weight transfer before bomb throw; grounded stance after detonation; distinctive stomp/land.
- Donut / caster: tail, ears and eyes convey arrogance/surprise; cast requires clear silhouette even when face is occluded; do not make magical attack solely color-dependent.
- Mongo / charger (later): long anticipation silhouette and convincing momentum; recovery cannot clip through solid terrain.

## Production strategy for AI agents

1. Define canonical silhouette, turnarounds/poses and a palette in editable source.
2. Create 4-frame idle and 6-frame run; select best image by side-by-side review.
3. Produce action frames using **the same master source**, not independent text-to-image prompts per frame.
4. Use Aseprite Lua to generate or modify repetitive details, tagged timelines, recolors, pivots, and export.
5. Generate contact sheet and a short loop GIF/MP4 using open command-line tooling; compare versions.
6. Flag style drift, jitter, clipped extremities, inconsistent shadows and ambiguous hit frames.
7. Require human approval for character silhouette and personality, even if tests pass.

## Known hazards and mitigations

**Inconsistent AI frames:** lock canonical master, palette and perspective. **Jitter:** validate pivot/foot contact across frames. **Scope explosion:** start with five tags; defer extra emotes. **Rigged pixel art artifacts:** never rotate single-pixel limbs across large angles without manual re-pixeling. **Animation and damage desync:** authoritative frame-independent attack timeline with event markers.

## Carl locomotion candidate (ANIM-CARL-01)

Draft, awaiting owner art review: [carl-locomotion-anim-01.md](carl-locomotion-anim-01.md). Pipeline note for integration: locomotion atlases may use a wider frame than the idle master as long as the pivot keeps the same foot-relative position (Carl: 36x56 / pivot 19,55 padded to 48x56 / pivot 25,55). Grounded tags keep the lowest opaque row on the pivot row; airborne tags keep the pivot on the physics foot line and may draw the feet above it. Loop / once / hold playback is recorded per tag in the manifest.
