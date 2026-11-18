# Combat feel and hybrid enemy physics — design contract v0.1

**Status: creative-direction APPROVED on 2026-11-17; numeric tuning and exact gameplay behaviors are NOT approved or implemented.**

Read [combat-first encounter direction](combat-first-encounter-design.md), [core gameplay](gameplay.md), [protagonists](characters.md), [destructible terrain](../03-tech/destruction.md), [hybrid solo and co-op progression](floor-01-gameplay-progression-v0.md) and [workorder instructions](../07-agents/workorders.md).

## 1. Approved combat identity

**THE CRAWL should feel like a grimy, exaggerated 1980s–1990s action film: fast, physical and spectacular.** Most ordinary enemies should die after a *small number of meaningful hits*, rather than sponge damage. Difficulty is driven mainly by threat mixes, behaviors, timing, number, level geometry and environmental hazards. Stronger monsters and bosses are deliberately distinct exceptions.

**Simple attacks are useful and fun; the most memorable kills emerge from interacting mechanics.** Long memorized input sequences are not the main source of mastery. Mastery comes from movement, timing, hero switching, knockback, selective destruction, and combining attacks with the terrain.

**Hybrid physics is the intended approach.** Player and enemy control, attacks, damage, collisions, authored environmental triggers and kill attribution should be deterministic/replayable. The visual motion can be expressive, exaggerated and partly authored; the game must not depend on a large uncontrolled ragdoll simulation for every enemy. Enemy bodies may collide with other enemies and breakable objects to create opportunities for cascading defeats **when prototype evidence justifies those rules**.

**Three tiers of satisfaction:**
1. **Direct:** a Carl punch/kick (or another valid available action) hits one goblin with compelling weight, sound, animation, knockback and clear kill confirmation.
2. **Physics/environment:** the same hit launches an enemy into a wall, another enemy, an edge or a destructible prop; the impact visibly changes the outcome.
3. **Authored chain reaction:** a hit or breakable support leads to a localized, legible collapse/explosion that affects multiple enemies. The chain is the consequence of the player's action, not a disconnected cinematic.

**Approved role distinction remains:** Carl delivers physical force and selective destruction; Donut jumps higher and farther and can exploit vertical positions. She is playable in co-op from the beginning, **without early magic**. After story-verified unlocks, ranged magic can amplify positioning and duo synergy. In solo, switching between heroes is a deliberate core mechanic, not a substitute for missing parallel paths.

## 2. Candidate physical vocabulary (prototypes, not commitments)

| Candidate mechanic | What the player sees | What must be proven |
| --- | --- | --- |
| **Hit confirm** | Brief distinct anticipation/contact, target impact pose and satisfying feedback | Readability and immediate response without overlong hit-stop |
| **Stagger** | Brief interruption of an ordinary enemy's attack | Fair counterplay; cannot permanently stun-lock an entire group |
| **Knockback** | Enemy is displaced with a clear horizontal/vertical component | Predictability, useful aim/spacing; not weightless and chaotic |
| **Airborne/thrown** | Enemy is launched or falls after a heavy hit | Stable interactions with platforms/walls and safe re-entry to ground state |
| **Enemy-to-enemy impact** | Thrown enemy interrupts/knocks down another | Meaningful spectacle instead of infinite collision/damage loops |
| **Terrain impact** | Enemy collides with a wall, brittle support or dangerous object | Visibly attributable environmental outcome, valid material handling |
| **Defeat aftermath** | Enemy is disabled/defeated, then death animation/debris resolves | Clear kill confirmation without corpses blocking critical routes or excessive physics |
| **Resistance/weight** | Larger/special enemies respond differently from fragile goblins | Telegraphs match rules; bosses are not trivially perma-launched |

**Not established:** exact attack count to kill each enemy, damage values, hit-stop duration, combo count, damage on corpse contact, launch angles, chain limits, aerial attack input, knockdown iframes, and whether defeated bodies remain gameplay-active. All require deliberate design and real controller playtesting.

### Hybrid authoritative / presentation split

- **Gameplay simulation, Rust fixed step:** hit resolution, attacker/source ID, attack tags, hurtbox overlap, deterministic impulse/velocity, damage, collision contact, active/staggered/airborne/defeated state, terrain events and protected-route checks.
- **Bevy presentation:** pose interpolation, squash/stretch frames, impact flash, hit-stop visualization, camera emphasis, original VFX/SFX, non-colliding cosmetic debris and optional simplified ragdoll-like *visual* finish.
- **Critical rule:** never let GPU particles or cosmetic ragdoll movement change authoritative kills, terrain, navigation, camera collision or input recording. Avoid full per-pixel material simulation. Keep ordered events, bounded impulses, bounded contact cascades and reproducible seeded traces.

An illustrative future event chain (not exact implementation API):

    AttackHit(attacker, target, tick)
      -> ApplyDamage(target)
      -> ApplyImpulse(target)
      -> ResolveContact(target, enemy | terrain | prop)
      -> OptionalContactEffect(target, hit_entity)
      -> OptionalTerrainDelta / authored collapse
      -> DefeatAttributedTo(attacker | source_event)
      -> EmitPresentationEvents

**Contact damage is NOT automatic:** a projectile body should not inflict damage on every overlapping enemy every frame. Decide eligible actor states, one-shot event identity, minimum impulse, immunity windows, deduplication and source attribution only after observing a basic prototype. Any default must be deterministic and safe.

## 3. Proposed first test action set

These are **test actions, not an approved final moveset**. Match current story capability rather than importing a late-book toolkit.

| Hero | Experimental move | Desired feel | Risk |
| --- | --- | --- | --- |
| Carl | Quick grounded strike | Immediate damage and believable scrappy effort | Bland/repetitive basic combat |
| Carl | Stronger kick / push | Intentional directional displacement into hazards | Everything becomes a knockback one-shot |
| Carl | Aerial contact attack | A risk/reward option when approaching from a platform | Invalid collision/pixel readability or excessive power |
| Carl | Selective break / context hit | Turn props and material weaknesses into lethal opportunities | Unclear which geometry is vulnerable |
| Donut pre-magic | Jump/high mobility + small physical strike or distraction | Genuine P2 tactical participation, staying alive and gaining positions | Becoming a passive spectator or implausibly strong cat |
| Donut after earned unlock | Aimed ranged spell / tactical follow-up | Reward vertical angles and tandem attacks | Unfair power scaling, uncontrolled screen-filling FX |

**No shotgun for Carl.** Do not silently grant Donut magic before the appropriate narrative event, and keep Shoulder Mode an explicitly unapproved experimental mechanic.

## 4. Enemy knockback classes (hypotheses, not static lore)

Do not make enemies distinguishable only by HP. Each archetype needs a discernible silhouette, tell, interaction with impulses and a useful tactical weakness.

| Candidate class | Example | Hit reaction idea | Player opportunity |
| --- | --- | --- | --- |
| **Light infantry** | Early basic goblin | Interruptible, small airborne throw | Fast direct kill; collision with prop/other goblin |
| **Ranged/area threat** | Bad Llama (later district) | Vulnerable during some tell/recovery | Close distance, reposition, manipulate sightlines |
| **Sticky/contact hazard** | Rot Sticker (later district) | Unusual wall/ceiling motion, not ordinary biped knockback | Deliberate trigger/avoidance; maintain distinct threat language |
| **Large machine** | Opening Dozer | Resistant/immune to normal knockback | Evasion, turning radius and environmental escape, not punching it to death |
| **Boss/heavy** | Juicer / Ball of Swine | Strong telegraph and conditional stagger, not unlimited launch | Discover timing/terrain interactions, resist cheap stun-lock |

**These are original gameplay proposals.** Check book evidence before turning any novel-specific action, weakness or visual element into asserted canon.

## 5. First playable combat fixture: COMBAT-PHYS-01

Do **not** design a full fighting system before testing one reliable contact chain. Reuse the existing separate WORK-021 greybox and only integrate approved production-scale graphics when source, camera and animation contracts are ready.

**Fixture 1 — single basic hit** *(first implementation in review: CP-01 PR, placeholder visuals; starting values in `src/feel.rs`, see [the scene doc](../03-tech/floor01-scene.md#combat-fixtures-combat-int-01-cp-01); the human verdict is still open)*:
- One Carl, one normal two-hit goblin placeholder, flat safe ground.
- Tune responsive attack action, readable tell, hit reception, small knockback, death feedback, restart.
- Compare current greybox baseline with updated hit feel; present real Bevy recording, controller evidence and short written critique.
- This must already be fun without destruction or explosions.

**Fixture 2 — controlled collision:**
- Same Carl and goblin plus a second goblin and one intact wall.
- Land one heavy knockback, observe contact with wall and other enemy.
- First test **stagger/positioning effects without contact damage**; add contact damage only if it demonstrably improves play and has unambiguous rules.
- Ensure event deduplication, same-tick ordering and no chain explosion/stun-loop.

**Fixture 3 — one spectacular environmental kill:**
- One breakable tagged support/prop with a clearly marked fall/hazard region, 2–3 goblins, protected exit.
- Player chooses simple melee against enemies OR causes a local collapse via Carl's currently available action.
- Tests must prove readable cause -> effect, deterministic material updates, valid hit attribution, enemy recovery/pathfinding and no softlock.
- An authored collapse (Level 3 structure) can be an independent later PR; if not yet safe, start with a simpler prop destruction/hazard.

**Fixture 4 — protagonist asymmetry / two humans:**
- A meaningful upper platform for Donut and a lower destructible attack lane for Carl; the encounter does not require two equivalent paths.
- Test one human swapping and 2P simultaneously; both must take active decisions.
- Donut's pre-magic contribution is positional/catlike, and post-unlock ranged follow-up is **a separate future fixture**.

**Design review gate:** record (a) direct-hit fun, (b) clear direction/causality and risk, (c) enemy physical readability, (d) kill attribution and recoverability, and (e) 1P/2P agency and framerate impact. A human must play; a green CI does not prove satisfaction.

## 6. QA contract and implementation boundaries

- Fixed-step deterministic gameplay, stable sorted collision/contact resolution, seeded scripted action replay and hashes including actor/terrain state.
- No repeated impact damage from standing overlaps or corpses; cap any future cascade by authored semantic rules, not GPU physics.
- Confirm no softlocks when breaking supports, trapping AI, changing platforms or temporarily removing normal walkable space; critical routes protected or recoverable.
- Tests cover simultaneous attacks, two enemies colliding, throw into a wall, invulnerability/stun state transitions, player restart and kill attribution.
- Input/gamepad checks: solo swap while an enemy is airborne, 2P attacks at the same tick, disconnect/rejoin during knockback without stuck commands.
- Compare 640×360 and 640×400 visual readability in actual Bevy. Do not claim Steam Deck performance from desktop/software Vulkan alone.
- Do not overlap agents on shared physics, collision or input authority types in parallel. Keep initial workorders small and reviewable.

### Decision ledger

**Approved:** combat-first over puzzles, quick lethal ordinary encounters rather than HP-sponge combat, expressively weighted physical hit feel, combos from interaction rather than long button sequences, **hybrid deterministic physics** capable of meaningful enemy/environment interactions, Carl/Donut contrasting combat mobility, antihero action-film tone, local 2P from the opening and switchable solo.

**Pending prototypes:** precise attack move set; per-archetype kill counts; whether airborne enemies deal contact damage to others; whether corpses can be thrown/used as projectiles; airborne collision angle tuning; camera shake/hit-stop budget; ragdoll-like presentation; combo UI and bonuses; stage-specific skills/unlocks; exact balance of spectacle vs readability.

**Out of scope for this design-only document:** code implementation, rewriting Bevy combat architecture, final sprite/sound production, full ragdoll simulation, 3P co-op, licensed content distribution.
