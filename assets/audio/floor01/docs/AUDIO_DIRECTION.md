# Floor 1 v2 — Audio Direction (provisional)

## Core goal

**Clarity and physical comedy over volume.** Reference the *feel* of clear arcade fighting-game hit confirmation and the physical chain reactions of 2D action platformers; do not copy or embed sound from existing titles. Keep the 1980s–1990s industrial action-cinema identity, without a constant din.

## Mix hierarchy (initial suggestions, validate with a sound designer)

- Music: low by default; never hide a hit confirmation. Allow up to ~3 dB sidechain attenuation for 100–180 ms on meaningful contacts; no permanent brickwall limiting of each event.
- Ambience: sparse and ignorable; make individual machines location-specific, and remove low persistent rumble when not in the chase.
- Kick swing: subtle, short and lower level than impact; still play on a miss.
- Kick hit: play **only on hit-confirm**, driven by collision/hurtbox events, and vary among at least 3 hits; keep no more than two overlapping major contact transients without priority limits.
- Goblin reaction: current breath-like sound is a **temporary substitute**. Actual goblin physical and vocal design requires more iteration.
- Enemy/environment collision: triggered by physics/environment events, not bundled inside the kick sample, so chains create readable timing.
- Heavy destruction: a distinct event, with occasional sub accent; avoid the same large crash for ordinary wall chips.
- Spatial blend: favor dry and near for character actions, distant/rolled-off for industrial beds; two-player local co-op shouldn't change the audibility of hit confirmations.

## A/B/C assessment order

1. Do we feel a kick *because* of timing + enemy movement, rather than loudness?
2. Is the game less fatiguing after 5–10 minutes of continuous fighting?
3. Can we distinguish a swing miss from a kick hit, an enemy-to-enemy collision, and a wall break?
4. Does musical tension work during the Dozer chase without competing with collisions or warning cues?

**Hypothesis:** Hybrid v2 is a good default to test, not a decision already ratified.
