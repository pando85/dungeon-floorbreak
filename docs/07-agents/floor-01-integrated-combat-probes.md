# Floor 01 — Integrated combat probes (first playable scene handoff)

**Status:** gameplay evaluation workorder / integration proposal, 2026-11-17. **Not implemented.** Intended to be included in the FIRST integrated Rust + Bevy playable scene under preparation, not built as an unrelated permanent demo.

**Owner instruction:** include the combat-feel and hybrid-physics experiments directly in the opening playable scene and its test scenarios. Preserve the story progression and avoid turning the chase into a disconnected training arena.

**Read first:** `AGENTS.md`, [opening encounter handoff](floor-01-opening-encounter-handoff.md), [combat-first encounter design](../01-design/combat-first-encounter-design.md), [combat feel/hybrid physics](../01-design/combat-feel-hybrid-physics.md), [Floor 1 integrated-scene tests](floor-01-integrated-scene-playtest.md), [existing combat greybox](../04-production/linear-combat-greybox.md), [current corridor scene](../03-tech/floor01-scene.md), [input and camera](../03-tech/input-camera.md), [rights/release](../04-production/rights-and-release.md).

## Integration decision

**One Bevy experience; two ways to test it.**

- **Normal story route:** short calm intro -> threatening Goblin Murder Dozer pursuit -> brief on-foot goblin encounter after escaping the machine -> retreat/reach the Tutorial Guild. The goblin encounter provides natural in-world proof of core melee feel, knockback and an optional environment-driven elimination. No on-screen debug scaffolding in this mode.
- **Developer/test scenarios in the SAME integrated scene executable:** deterministic, explicitly named fixtures for hit, throw/other-enemy impact, breakable prop, possible defeated-body impact, and solo/2P authority. Scenarios set up otherwise identical actors, collision/terrain contract, sprites and A1 rendering, then capture gameplay states. These are controlled test configurations, **not** extra compulsory introductory rooms.

The human owner should be able to start normal play and separately reproduce a specific experiment in one or two commands, using documented CLI scenario names selected by the integrator. **Do not prescribe CLI flags as implemented until the agent adds and tests them.**

## Narrative and ability constraints

- The opening Murder Dozer is a **survival/escape threat**; normal unarmed combat does not defeat the vehicle.
- The subsequent on-foot goblin skirmish is a small first opportunity to fight, not a full goblin workshop.
- **Pre-Guild Donut has no magic**. She is intended to be a playable co-op partner from the beginning; until WORK-005 / WORK-006 are implemented, mark solo-swapping and 2P cases **N/A / pending**, never pretend passive follower behavior validates them.
- Carl should use only movement, physical combat and scenery interactions that are justified at this story beat. **Do not hand him time bombs/explosives early because the separate WORK-021 greybox has them.** Move such demonstrations to test-only post-unlock scenarios if appropriate, or wait for the relevant section.
- Keep the approved Carl 52 px / Donut 40 px editable masters and scale, A1 lighting, 640×360 / 640×400 virtual resolutions, 1980s–1990s scrappy action antihero tone. Do not use a shotgun.
- The native asset pipeline must be used for production sprites; placeholder telegraphs are acceptable **only as disclosed mechanical probes**, not as final approved art.

## Four initial combat probes

### CP-01 — Basic goblin takedown (must have)

**Fixture:** Carl, one regular goblin on clean ground; no special prop or enemy collision distractions.

**Player observation:** Does a simple strike feel immediate, heavy, readable and worth repeating? Does the goblin react with visible stagger/knockback and a clearly identifiable defeat?

**Acceptance evidence:** input-to-hit and animation timing, fixed-step replay, no double damage from a held input unless defined, full kill + fast restart, real Bevy screen recording, human verdict on impact/recovery/clarity. Keep two-hit ordinary goblins from WORK-021 as a **baseline to compare**, not immutable balance.

### CP-02 — Knockback into another goblin (must have)

**Fixture:** Carl, two goblins close together, safe wall/level geometry, controlled stronger kick or push *as a test candidate*.

**Player observation:** Can one enemy be launched into another, with both contacts intelligible? Is it fun without requiring a long input sequence?

**Implementation sequence:** first collide and **stagger/reposition** the second goblin with **no automatic collision damage**. Add impact damage behind a test variant only if feedback warrants it. Deduplicate per event and stable-sort contact resolution. No per-tick repeated damage, infinite chains or target-through-wall exploits.

**Acceptance evidence:** deterministic replay for contact event order; two enemies respond predictably; camera can track flight without losing the player; fixed-time impacts and clear player attribution.

### CP-03 — Environmental elimination (must have)

**Fixture:** one identified breakable prop or support in the post-chase on-foot encounter, 2–3 goblins and a protected onward exit.

**Player observation:** Carl can choose direct combat OR a deliberate action that causes the prop/terrain to harm an enemy; the player understands the causal chain and wants to try it again.

**Important distinction:** breaking a *small already-supported prop* may be the first integrated demonstration. A dynamic collapsing support graph is later L3 terrain technology and must not be claimed delivered before it exists. If breaking a prop cannot be supported at this opening moment without granting later-story equipment, use a plausible melee-interactable fragile element.

**Acceptance evidence:** deterministic hit/prop/defeat chain, visible enemy reaction, nav/collider update and exit reachable under alternative destruction/event order. At least one genuinely replayable alternative to the direct kill.

### CP-04 — Defeated body impact (optional hypothesis; NOT a required opening feature)

**Fixture:** same as CP-02 but with already defeated airborne goblin; single additional target or breakable object.

**Open question:** Is a corpse-like defeated enemy still a useful physical projectile after the kill, or does that confuse the distinction between active combat and cosmetic death feedback?

**Experimental variants:**
- A: defeated bodies have **presentation-only** motion and no gameplay contact.
- B: one short, capped physical aftermath phase with **one authoritative impact event maximum**, then become nonblocking cosmetics. No repeated overlap damage, dangerous corpse piles or excessive ragdolls.

**Acceptance evidence:** compare player clarity and enjoyment A/B on same setup. If little added value, keep A and defer physical corpses. This is **not** a blocker for first playable scene completion.

## Solo and co-op cases: same fixture, gated by real implementation

- Initially integrate CP-01/02/03 with the actual current engine limitations and correct character/story state. **Do not delay all combat tests** until fully functioning multiplayer and companion AI exist.
- After direct Donut control / 2P device authority is implemented, replay the same fixtures with **P1 Carl and P2 Donut**. Donut can leverage higher/farther jumps and catlike physical/positional contributions; no pre-Guild magic.
- After solo character swap and companion commands exist, compare same scenario with **one human switching between Carl and Donut**, verifying NPC/companion recovery.
- Record which scenario variants are executable on each PR. Do **not** add a fake pass to a missing co-op feature.
- **Shoulder Mode remains a separate optional experiment** and is not needed to validate these impact tests.

## Integration structure and regression boundaries

1. **One source of truth for combat:** reuse the fixed-step domain logic / damage / material model from WORK-021 wherever compatible. Do not maintain an independent toy knockback system in the integrated scene alongside an unrelated production combat implementation.
2. **Stable test scenario setup:** deterministic world seed, spawn positions, initial enemy state and identifiable script/action trace. Scenario IDs are data/test contracts, not necessarily distinct Bevy binaries.
3. **Separate gameplay from presentation:** hit/impulse/contact/damage/cascade state in deterministic Rust logic; Bevy animation, sprites, lights, sound and non-colliding VFX cannot alter simulation outcomes.
4. **Safe environment:** protected critical floor/exit, no unstoppable machine effects, bounded collision cascading, stable enemy reaction and intentional camera limits.
5. **One integrator owns shared scene composition.** Parallel agents can work on pure gameplay tests or art sources only with agreed interfaces; **do not edit the same combat schema, collision rules or `floor01_scene` composition simultaneously**.
6. **Prefer small PRs:** combat scene integration -> goblin hit feel -> thrown-enemy impact -> prop kill -> optional defeated-body probe -> true 2P/solo swap evaluations as their foundations arrive. Ownership/rebase to current main required.
7. **Performance:** measure real Bevy behavior at 640×360 and 640×400, optionally output 1920×1080; report test hardware, build and frame intervals. Use CI Xvfb/software Vulkan for regressions but never claim Steam Deck performance without actual measurement.

## Required CI and human evidence

- `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, plus project-specific asset/scene validation.
- Headless deterministic replay: one normal hit; two sequential hits; throw into wall; throw into enemy; prop hit -> enemy defeat; repeated contact must not cause repeated unintended damage.
- Protected exit remains reachable following CP-03 and after restart; old corridor camera and noncombat gameplay remain unchanged where no combat is active.
- Real Bevy screenshots or short recordings: native-scale actor/enemy impact readability, enemy flight, one environmental kill, no clipped or unlit critical cues.
- Human controller review of normal story play **and** debug fixture: answer whether (a) direct kill feels good, (b) knockback makes the next combat decision more interesting, (c) environment kill is understandable and satisfying, (d) Donut matters when actually controllable, (e) the post-chase fight preserves the opening pace.

## Readiness order

**P0 in first scene:** CP-01 and CP-02 (basic melee and controlled knockback).  
**P1 if supported safely by the opening scope:** CP-03 (one environment interaction with a clear kill).  
**Future research:** CP-04 (defeated-body projectile), post-unlock spells/explosions, Shoulder Mode and extra enemy classes.  
**Required later co-op gate:** repeat the same CP scenarios with actual solo-swap and 2P controls, rather than designing another disconnected combat arena.

The goal is a **playable opening that teaches and proves the core combat fantasy**, not a long showcase of internal mechanics. No gameplay implementation is claimed by this workorder document.
