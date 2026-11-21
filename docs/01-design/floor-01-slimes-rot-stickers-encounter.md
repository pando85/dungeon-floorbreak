# F01-SETPIECE-HAZARDS — "The Floor Lies" (Slimes / Rot Stickers)

**Status:** original gameplay encounter direction approved for detailed design; **this document is a gameplay hypothesis and implementation handoff, not a playable level**. Existing Slime and Rot Sticker **visual concepts** are approved, but their exact gameplay implementation, timings, contact damage and Bevy integration need review.  
**Placement:** Floor 01, Act II / proposed Stage 06, after Goblin Workshop and before the survivor camp and Juicer. Carl's earned dynamite, Donut's post-Guild Magic Missile / existing physical attacks are available. No Puddle Jumper until after Juicer.

**Memorable promise:** you learn to read dangers on **floor, wall and ceiling**, then discover you can make two of those dangers collide with your enemies. The sequence is not an unavoidable spike hallway or a special-ability platforming puzzle.

## 1. Authored encounter grammar

Build **one linear scene with three short connected pockets**, each initially readable and mechanically distinct. First target length **2–3 minutes** played at speed; any extra labyrinth/navigation content belongs in a *separate* Stage 06 scene.

- **H-A — Slime floor:** teach one corrosive ground hazard and its visible, safe alternate footing in an uncomplicated combat pocket. One slime at first.
- **H-B — Rot Sticker walls/ceiling:** teach an explosive small dark wall/ceiling-traversing entity in isolation, **not** a conventional huge spider or a goblin bomb.
- **H-C — The crossover:** one slime, one small mobile foe/ordinary goblin if gameplay supports it, and **1–2** Rot Stickers. Player can cause a Rot Sticker to detonate near an enemy through legitimate contact/attack, or simply fight normally and avoid threats.

These must remain **distinct species**. Source descriptions make Rot Stickers small dark rounded wall/ceiling creatures that explode on contact; slimes are separate corrosive threats. **Do not invent slime splitting** or assume Rot Stickers spawn packs from a distant hidden nest.

## 2. Geometry: distinct safe paths with optional vertical choices

Start with ~**960–1152px** wide authored scrolling room, single protected floor exit, normal 640×360 / 640×400 camera rules and native hero source sizes. Layout dimensions are prototypes, not fixed LDtk values.

| Span | Space and danger | Design goal |
| --- | --- | --- |
| Entry x~0–160 | Dry stone, visible corrosion stain further right; no hidden hazard | Teach the visual language before damaging the player |
| A x~160–400 | One short slime area with a **safe continuous or short-hop bypass** and broad landing pockets; ordinary enemy at a fair distance | Attack / avoid from the ground or elevated foothold |
| Transition x~400–500 | Dry, calm corridor segment | Reset attention before ceiling threat |
| B x~500–720 | One/at most two Rot Stickers on obvious overhead surface, safe retreat pocket and clear descent/trigger silhouettes | Look above and time movement |
| C x~720–1008 | Wider combat pocket, short upper ledge, separated hazards and at least two safe zones | Use one threat against another; no mandatory timed detonation |
| Exit x~1008–1152 | Protected dry floor / small checkpoint transition | Safe regroup regardless of hazard state |

No fully covered floor corridor where toxic slime forces players to take damage. No Rot Sticker or blast origin directly above spawn/checkpoint/only exit. Do not require double-jump, newly added Donut leap tuning, Puddle Jumper or two simultaneous player positions. Short upper shelves must not enable a permanent immunity turret or fail co-op camera/rejoin tests. Vertical threat must be visible before it enters an attacking range; offscreen ceiling spawns are unacceptable.

## 3. Environmental/enemy behavior contracts (initial tuning hypotheses)

### Slime: corrosive floor threat, *not* automatically a bomb

**Gameplay minimum:** `idle` → `approaching` → `corrode_tell` → `corrosive_contact` → `recover` → `defeated`.

- One or two localized acidic footprints with distinctive **green surface grammar** and honest collision/hurt bounds. Clear short advance/ooze anticipation **~24–36 fixed ticks** before first dangerous contact; no invisible instantaneous slime puddles behind scenery.
- Proposed first damage: **1 HP per qualified contact**, gated by current actor invulnerability/hit cooldown; no per-tick rapid drain. Environmental slime puddles, if separately authored, need an explicit visible decay/clear state and bounded application tokens. No indefinite automatic area saturation.
- Slimes can be killed by existing attacks with observable hurt reaction; do not assume a split-on-hit mechanic (visual caption is not authority). Their contact knockback, mass/resistance and ability to be shoved toward other hazards require gameplay approval and tests rather than guesses.
- Avoid neutralizing every slime instantly with a distant Magic Missile so that neither actor needs to think; tune their position, velocity and response, not generic boss-level HP.

### Rot Sticker: small traversal explosive

**Gameplay minimum:** `latched` → `scuttling` → `attack_warning` → `detonating` → `spent`.

- Separate a **readable traversal cue** (dark rounded creature, wall/ceiling motion, surface tick sounds) from a **detonation warning** (contrast pulse, sharp click or distinct sprite stretch). Proposed warning **18–24 fixed ticks**, to be confirmed against real movement speed and player's reaction time.
- Blast area initially ~**40–56 world px** radius, consistent visual telegraph/hitbox semantics and existing explosive damage channel. Radius/tell/time fully data-driven; use actual deterministic proximity/contact detection.
- Only genuine triggering events detonate: contact inside the designed danger envelope, authored attack interaction or eligible actual physical collision. No scripted global trigger if a goblin walks beneath an arbitrary x coordinate.
- Exactly **one detonation per entity**, one damage evaluation per victim/blast token. Proposed first harm **1 HP for nearby normal enemies and heroes**. Do not give every environment explosion free instant kills by default.
- Players may trigger one early using already available **Magic Missile or dynamite only when their existing collision rules actually support it**. Respect projectile/material behavior; no homing shots just to create a cool outcome.
- Rot Sticker should be dangerous on the underside/edge of a perch as well as above the ground; a visually tiny creature may never be an invisible hitbox. Limit simultaneous active stickers to 1 in B, 2 in C for the first playtest.

### Hazard-vs-hazard interactions

A Rot Sticker blast may legitimately damage a slime or adjacent ordinary enemy **only through the standard explosion/collision pipeline**. Whether an affected slime produces a briefly larger/shorter toxic patch is a **later optional experiment**, not part of initial definition. Do not introduce chain-splitting, extra acid fireballs, force-field props or explosion cascades to the first version.

In C the player should have at least three possible clear patterns:
1. direct combat while keeping out of hazard envelopes;
2. safe positional evasion, letting hostile contact/detonation occur as a tactical opportunity;
3. active Magic Missile / dynamite / physical push triggering a real blast near an enemy.

If the encounter can only be cleared with the third path, redesign it as optional.

## 4. Three-beat dramatic staging and learning

| Beat | What the player sees | Decision taught | Failure signature |
| --- | --- | --- | --- |
| A — Don't step there | Corrosive surface cue, one slime with low attack density | Alternate footing and standard attacks work | Hazard camouflaged / contact before warning |
| B — Look up | An overhead Rot Sticker traverses a distinct wall/ceiling track | Observe, redirect or safely outrun detonation | Offscreen drop, tiny unreadable enemy or RNG |
| C — Turn danger into advantage | A goblin/slime wanders close to a moving sticker while player has a clear line | Time a hit, position foes, provoke collateral explosion | Forced exact frame order or scripted enemy teleport |
| Out — Breathe | Clear, dry exit and a moment without both active hazards | Feel deliberate mastery | More damage after scene is already over |

## 5. Camera, controls and dodge interaction

- Camera normally tracks forward and leads upward only enough to include dangerous ceilings, without hiding floor acid. Shared 2P camera must display **both** a floor danger and overhead bomb tell when a threat is active.
- Do not add crouch, evade or special platforming controls to solve the hazards. Base jump, movement, current attacks and unlocked special remain valid.
- Donut's automatic **12-second dodge/reflect** is being implemented by another local agent. This set-piece must not duplicate or rebalance it. Each hazard/attack has explicit `dodgeable` classification when the domain supports it. **Proposed starting rule for human review:** persistent acid contact and a Rot Sticker AoE are environmental hazards and should **not** be consumed as an ordinary dodge of a targeted melee hit; however, confirm with the independently approved dodge design before integrating. Show clear distinction between dodgeable targeted attacks and nondodgeable area hazards.
- Existing hit cooldown / damage rules, mana resource and companion rules are authoritative. No new buddy navigation/commands in this encounter task.

## 6. Original art/technical boundaries

- Source visual identity: `docs/02-art/floor-01-enemy-concept-roster.md`, `docs/02-art/floor-01-late-enemy-boss-concepts.md`, `docs/02-art/floor-01-art-atlas-v1.md`. Exact production native enemy dimensions/frames **not** approved by concept images.
- Keep **separate slime-green floor and tiny black ceiling markers**. Their effects must not blend into a generic fluorescent-green contaminated tunnel. Base Floor 01 architecture is brick/stone; local corrosion is restrained.
- Keep existing authoritative collision/material/hero source rules; prefer one new generic explosion interface only if the existing dynamite/terrain event system cannot represent the sticker's cause and effect.
- Terrain destruction is not mandatory here; do not implement a novel reusable wall-break engine just for this room. Any drops, safe ledges or exits are protected.

## 7. Deterministic acceptance fixtures

| ID | Test | Required result |
| --- | --- | --- |
| HAZ-01 | Walk A with readable safe path | Avoid slime without damage using base moves; exit reachable |
| HAZ-02 | Direct attack on slime | Ordinary kill possible, no unapproved splitting or invisible acid |
| HAZ-03 | Rot Sticker traversal & detonation | Correct wall/ceiling movement, visible tell and one damage event per blast/victim |
| HAZ-04 | Magic Missile / earned dynamite interaction | Actual projectile/blast collision required, no script-on-press kills |
| HAZ-05 | Goblin nearby vs outside Sticker radius | Correct selective hurt/defeat and environmental source attribution |
| HAZ-06 | Leave all hazards untriggered | Safe regular-combat exit, no mandatory environmental combo |
| HAZ-07 | Multiple orderings of C | Same deterministic result for each fixed input/seed, no infinite chain |
| HAZ-08 | Restart mid-ooze/latched/scuttle/explosion | All state, consumed stickers, hazards, timers and actor HP reset coherently |
| HAZ-09 | Solo both active heroes, 2P local | Same route, no human-P2 gate, readable camera and regroup |
| HAZ-10 | Captured live input and replay | State hashes, hazard classification and event order reproduced twice |

Manual evaluation: can the player infer hazards without reading HUD text? Are slimes/stickers tactically distinct? Does the first accidental enemy kill feel like a discovery? Are the last 30 seconds still fun with no active stickers? Keep initial simultaneous count small; tune from recorded attempts, not assumed difficulty.

## 8. Dependency-aware implementation order

1. **HAZ-A:** data-authored safe geometry and two independent **isolated** enemy prototypes using existing engine primitives; no mixed corridor yet.
2. **HAZ-B:** native-sized visuals/clear tells and verified contact/damage/hazard classification.
3. **HAZ-C:** mixed pocket with optional enemy-vs-explosion interaction and source attribution, bounded effects and deterministic replay.
4. **HAZ-D:** native art review, companion/2P actual integration and human playtests. Preserve consistent 60Hz simulation, fast checkpoint retry and no path softlocks.

Do not start all three at once if the enemy behaviors are not yet integrated; implement and validate each before combining. This PR is **design-only**. The future agent should create a **separate implementation PR** and confirm current branch ownership, actual enemy availability and collision/Playtrace infrastructure.

**Key related docs:** `docs/01-design/combat-first-encounter-design.md`, `docs/01-design/floor-01-gameplay-progression-v0.md`, `docs/01-design/asymmetric-traversal-and-gates.md`, `docs/03-tech/playtrace-01-session-capture.md`, `docs/03-tech/level-data.md`, `docs/02-art/floor-01-enemy-concept-roster.md`, `docs/05-decisions/ADR-0010-combat-first-progression.md`.
