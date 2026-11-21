# FLOOR01-BOILER-01 — Goblin Boiler, signature combat set-piece

**Status:** owner-selected design direction; **implementation brief / gameplay hypotheses**, not an implemented room or approved balance. This PR intentionally contains **specification and agent handoff only**. Final acceptance requires actual in-engine playtest.  
**Location:** Floor 01, Act II, Goblin Workshop (proposed Stage 05). Carl has already earned dynamite; Donut has already eaten the Guild biscuit. This room must not grant either ability or bypass narrative progression.  
**Pitch:** "I kicked a goblin into a valve, the pipes screamed, and a piece of machinery crushed the others." An industrial accident **authored by the player**, never an automatic cutscene or a forced switch puzzle.

## 1. Creative and gameplay contract

- **Arcade flow, not a metroidvania:** enter from the left, move right, fight, exit. One short side-height option; no mandatory backtracking, side quest, item hunt, or all-enemies-wave gate by default.
- **Three satisfying approaches:** normal punches/kicks/magic; tactical one-prop exploitation; an optional two- or three-link environmental chain. None is the sole solution. Ordinary direct kills must still feel good.
- **One visible industrial identity:** familiar Floor 01 masonry supporting a localized ramshackle goblin boiler works; heat, steam, rattling copper, hanging iron. Keep surrounding catacombs dry and recognizably Floor 01, not an entire lava/sewer/neon biome.
- **Mechanical distinctness:** kick/launched enemy can strike a vulnerable fixture; dynamite can break a brace; steam displaces/interrupts; falling weight punishes positioning. Environmental contacts are authoritative, bounded and attributable.
- **One human playing either hero and genuine 2P share the same room**. No mandatory paired-button timing, AI companion behavior rewrite, shoulder mode, special loot, arbitrary new controller button, invulnerability, or enemy-only magic.
- **Design non-goals:** a simulated fluid-pressure solver, general rigid-body stacks, unbounded shrapnel, a new enemy type, scripted mass death, bespoke permanent level destruction engine, viewer/sponsor unlocks.

## 2. Spatial brief — measurable *greybox starting hypothesis*

Native scene uses existing 640×360 / 640×400 virtual resolution and approved 52px Carl / 40px Donut sources. Coordinates below are **illustrative world-pixel authoring targets** (x increases right, y increases down), to be reconciled with existing LDtk/material walking-line conventions and actual jump arcs. Do not paste them as definitive colliders without engine-side reachability proof.

| Part | Suggested world geometry | Purpose / constraint |
| --- | --- | --- |
| Room envelope | about **1152 px** long (72 × 16 tiles), scroll camera; ground walking line near **y=304** | Approximately 1.8 native screens; continuous protected ground lane across the room |
| Entry/safe read | x=0–224; Carl spawn ~x=112, Donut ~x=80 | First enemies stay passive/out of reach until the mechanism is visible; allow 1–2 seconds of comprehension |
| First fight | x=224–496 | A plain opening exchange; no instant hazard auto-kill |
| Boiler backdrop | x=496–672 behind player collision lane | Huge readable visual landmark; **not** an impassable central wall |
| Left elevated path | x=304–448, top roughly y=240 (64px rise) | Optional attack/positioning; reachable by actual abilities; no static invulnerable turret position |
| Pressure valve | x≈464–496 near ground height | Clearly marked, vulnerable, accessible to punches/kicks/projectile impact; not required to exit |
| Outlet / steam lane | starts ~x=688, extends right toward ~x=768; height/range data-driven | Visual anticipation and horizontal displacement, not an invisible instant-kill beam |
| Hanging weight | footprint initially around x=784–848; hanging above right-side ground pocket | Apparent risk from above, shadow warning and single fall |
| Breakable anchor | low accessible tether/brace near x≈744–776 | Can be weakened by separate melee hits, launched body or explosive; not hidden near the ceiling |
| Optional small fuel drum | x≈848–880 on far side of weight | Secondary bounded reaction if struck directly, blasted or genuinely contacted by weight |
| Right elevated path | x≈928–1056, top roughly y=240 | Optional repositioning and enemy pressure; provide normal ground exit |
| Exit/recovery | x=1056–1152 | Protected, always reachable; no falling weight, drum or rubble may seal it |

Suggested topological sketch (not pixel-authoritative):

```text
                    [left upper perch]                [right upper perch]
ENTRY ── goblins ────┐                            ┌────────────┐ ─── EXIT
                      VALVE     BOILER      BRACE | WEIGHT ↓  DRUM
GROUND ===========================================================>
                     <<< steam lane >>>     fall footprint
```

**Spatial rules:** the ground line remains connected; visual boiler depth is a backdrop with localized interactive colliders; never put a giant opaque foreground pipe across Carl/Donut or the blast tells. Both upper perches have safe return to the common path. Spawn, exit, checkpoint and essential path geometry use protected materials. Do not guess native scale from an upscale to 1080p.

**Camera:** scroll with player(s); introduce the boiler/weight before they become lethal, bias/lead toward an active telegraph, keep two heroes and major tells legible. Do not adopt a new noninteger zoom policy or force an ADR conflict. For any proposed camera change outside current contracts, stop at a testable proposal.

## 3. Encounter dramaturgy and enemy contract

| Beat | Trigger and pressure | Player learning / sensation | Exit condition |
| --- | --- | --- | --- |
| B0 — Read the machine | Arrive x~128–224; short readable vista, background boiler motion | See valve, tether/weight and two combat lanes before danger; no forced dialog | Move onward |
| B1 — Ordinary brawl | Two common goblins **staggered**, not an instant synchronized pincer; one approaches the left lane | Direct kill, punch/kick, interrupt, launch; normal combat viable | Live combat |
| B2 — First accident | If player/foe strikes valve or anchor, the room reacts. If nobody does, the fight proceeds normally | The player caused the response; no pre-authored mandatory trigger | Live combat |
| B3 — Escalation | One later engineer (when its gameplay is merged and tested), otherwise a common goblin; optional existing Bomb Bard **only** when its independent gameplay has been reviewed | More pressure, readable angle to the anchor/weight, discover the second link | No more than 4 living enemies simultaneously in the default initial probe |
| B4 — Resolution | Remaining enemies defeated or bypass condition agreed in integration; floor rules should not require a failed prop-chain puzzle | Optional spectacular aftermath; distinct damaged-room silhouette | Exit reached, checkpoint and next encounter |

**Initial enemy placement hypotheses:** common goblins around x≈344 and x≈640; later reinforcement x≈896. A fourth upper-platform enemy is optional only if its attack, traversal and camera visibility are separately implemented and vetted. The **Goblin Engineer is already a separately proposed CF02-dependent type**; never fabricate AI or art to fill a slot. Use existing goblins for the first playable baseline. Avoid static ranged enemies who cannot be attacked from their perch, spawn into damage volumes, or unreadable offscreen fire.

An upper route is not a required Donut-only corridor. She should gain useful lines and safety/risks there with already earned Magic Missile and pounce. Carl must still have physical access to the ground route and environmental interactions. No bespoke companion navigation logic in this work; verify against the concurrently implemented companion systems and mark blocked cases honestly.

## 4. Interactive props: small explicit state machines

Every prop is a generic, data-authored entity with an authored ID, position, valid collision/attack channels, typed transitions, activation source and capped events. Reuse existing support, damage, material, hazard, combat event and explosion mechanisms where possible. Only add the narrowest shared domain primitives if the repo has no equivalent.

### P1 — Pressure valve / steam vent

- Visual states: `intact` → `ruptured_warning` → `venting` → `spent`; busted pipe and low residual steam remain visibly damaged for the attempt.
- Trigger by **distinct landed physical attacks, an actual sufficiently energetic launched enemy impact, or a dynamite blast** at its authored vulnerable fixture. A normal enemy merely walking into the valve never triggers it.
- Default hypothesis: one strong qualifying impact or 2 distinct light strikes. At activation, **24 fixed ticks** of audible/visible pressure warning, then **72 ticks** of bounded venting (~1.2 s) in an explicit horizontal area. These are tuning knobs, not lore.
- Steam is a contact hazard with a unique activation token: **at most one authoritative damage/impulse application per target per burst**, optionally refreshing a non-damaging push in a carefully validated interval. Proposed first numbers: 1 HP enemy / 1 HP player on first valid contact, modest rightward impulse, ordinary enemies stagger. All actors can see and avoid the tell; never use a hidden instant AoE. No friendly-fire exception from the world hazard unless existing policy expressly requires it; record results.
- Subsequent direct hits do **not** create infinite repeated vents in v1. Broken art persists; no pressure accumulation simulation.
- Allow steam to interact with the specific damaged hanging-anchor fixture if the geometry/connector is explicitly authored and the brace is already weakened; this can create an **optional** second link, but steam alone must not always drop the weight. Implement the simple independent valve first.

### P2 — Hanging weight and breakable anchor

- Author one low, clearly readable vulnerable brace/tether; reuse CP-03 distinct-hit support rules if feasible. Proposed: **2 distinct melee hits** or **1 actual dynamite blast**; launched-body impact may qualify under a declared impulse threshold, once per launch token.
- After the final qualifying hit, enter `released` with a **24-tick falling-warning** (creak, swinging chain, footprint shadow), then a fixed-step gravity drop and **one** swept impact against the bounded ground footprint. Start with the repo's gravity/impact primitives, not custom full-rigid-body physics.
- One landing event; hurt/defeat each overlapping hero/enemy once with source attribution, no double hits from resting collider. Initial proposal: lethal to ordinary goblins directly under the footprint; player loses at most 1 HP for this hazard; armored/special reactions are enemy-profile dependent.
- After landing, weight crushes into a **low 32px rubble profile** (or equivalent protected, traversable data-authored remnant) that does not block the only exit. It visibly changes sightlines/spacing; both heroes must traverse or safely route around it in solo and 2P.
- No magnetic pull, fake hitbox extensions, or scripted steering of enemies underneath the weight.

### P3 — Small secondary fuel drum (optional after P1/P2 pass)

- One drum maximum in first room variant; `armed` → `telegraph` → `detonated`, no reset until attempt restart. Cause: actual blast, falling weight impact within authored contact area, or real qualifying attack.
- Blast may trigger nearby **eligible** interactables through the ordinary damage pipeline, not unconditional global scene switches. Distinct area tell before harmful activation; initial effect must not overlap the spawn or protected exit.
- **Bounded chain rule:** max one activation per prop per attempt, deterministic contact ordering and chain depth bounded to 3 authored links. No double-spawn, overlapping-damage-per-tick, infinite propane/steam loops or unbounded debris.

### P4 — Boiler landmark

The primary boiler is the **large background visual identity**. It rattles, changes light/pressure animation, and receives a damaged-state silhouette when connected pieces break. **Do not add an independent fourth lethal boiler explosion in v1.** A catastrophic-looking *presentation* may accompany a genuinely caused P1→P2→P3 chain, without additional gameplay hitboxes/damage or map-wide scripted kill. Distinguish decorative pixels/VFX from collision and damage.

## 5. Player-authored solutions, not a scripted puzzle

| Route | Input chain | Observable outcome | Validity |
| --- | --- | --- | --- |
| A — Straight fight | Ordinary punches/kicks, Donut Magic Missile/pounce, movement | All combatants can be defeated without hitting any prop | **Required**, test in a props-disabled baseline |
| B — Tactical accident | Kick/launch a goblin into the valve, or strike it normally; fight through one steam burst | Enemies stagger/reposition; player uses altered space to win | **Required**, player-origin and enemy-origin variants |
| C — Big chain | Break/soften anchor and vent valve at a useful moment; drop weight near enemies and optionally trigger drum via **real collision/explosion** | Attributed sequence with distinct tells, falling mass and limited aftermath; should feel dramatically better but not compulsory | **Required** P1+P2; P3 is an optional later milestone |
| D — Unplanned variation | Dynamite directly weakens anchor/drum; enemies do not occupy footprints | Visible changed room but ordinary combat is still needed | **Required**: no automatic stage-clear guarantee |

**Avoid fake spectacle:** if enemy-to-enemy collision is presently interrupt-only and 0 damage (CP-02 baseline), **keep it**. Enemy-to-prop interaction can have an independently tuned qualifying impulse without secretly activating enemy-on-enemy contact damage. The objective is attributable physical consequence, not damage inflation.

## 6. Determinism, progression, restart, replay and camera

- Authored state only changes in fixed 60 Hz simulation; VFX, lighting, sound, hitstop and camera animations cannot decide collision or damage.
- Stable prop IDs, activation IDs, launch IDs and source actor/team attribution; event examples `PropDamaged`, `SteamStarted`, `BraceReleased`, `WeightLanded`, `ChainTriggered` and `EnvironmentKill`. Reuse/adapt existing event schema; **do not silently force a breaking Playtrace schema change**.
- Capture complete mutable room state (including prop state, active hazard windows, one-shot damage tokens, broken material and hanging/landed geometry) in authoritative state hash, reset descriptor and replay snapshots as appropriate.
- Live playtest attempts are recorded by default per PLAYTRACE-01; reproduce the physical chain in `playtrace verify` and render-replay when supported. If a recorder integration is pending, the code must still expose deterministic fixtures; explicitly report the missing gate instead of claiming ML readiness.
- Restarting from the encounter checkpoint restores initial enemy/prop/terrain state and same seed; **narrative unlocks remain earned**. Checkpoint before the action, never inside a lethal chain.
- Protected exit and walkable path, co-op regroup, no crush softlock, no destructible path loss; simulate both active-player identities and true 2P. The camera must not hide a falling warning, steam burst or newly entering enemy.
- Avoid unnecessary rewrites to `src/bin/floor01_scene.rs`, `src/combat_scene.rs` or shared physics while parallel Carl/Donut progression, companion, terrain and replay PRs operate. Coordinate shared-file owners before edits.

## 7. Acceptance matrix (headless and real play)

| ID | Repeatable scenario | Machine assertion | Human evidence |
| --- | --- | --- | --- |
| BOIL-01-DIRECT | Fixed seed, all props untouched, clear goblins | Exit reachable, at least one ordinary kill, no prop activated | Direct combat remains satisfying |
| BOIL-02-VALVE-MELEE | Carl hits valve with ordinary attack | One warning, one vent, one spend; distinct-hit rules honored | Valve and spray clearly readable |
| BOIL-03-BODY | Carl launches living goblin into valve with qualifying impulse | Physical collision (not proximity script) triggers, once per launch ID | "I caused that" unambiguous |
| BOIL-04-THROW | Carl uses **earned** dynamite on brace | Release only when capable, one throw, correct damage/source | Great wind-up/throw and outcome |
| BOIL-05-WEIGHT | Drop with one enemy under footprint and one just outside | Only overlap hit at actual landing tick; one death attribution | Shadow/tell gives an honest dodge window |
| BOIL-06-CHAIN | Authored P1→P2 (and P3 when present) on fixed seed | Stable causal order, bounded events, no duplicate damage or infinite activation | Chain is spectacular and intelligible |
| BOIL-07-SAFE | No-hit, melee-only, props-left-broken and rubble-settled variants | Exit and both hero regroup paths stay reachable in each variant | No frustrating jumps or opaque blockers |
| BOIL-08-RETRY | Death midway through steam/fall then retry | Reset fixtures, timer/prop/hash and seed; story capability flags retained | Fast restart; no repeat narrative |
| BOIL-09-PARTY | Solo Carl, solo Donut, and real local P1+P2 | Same critical progression, no P2-only gate, no controller cross-talk | Every player has something useful to do |
| BOIL-10-TRACE | Record real run; replay it twice | Event ordering/state hash identical; damage event attribution preserved | Real tick-to-frame correspondence |
| BOIL-11-CAMERA | 640×360, 640×400, 1080p output and handheld-relevant size | No player/tell outside intended camera bounds during authored variants | Both heroes and all telegraphs readable |
| BOIL-12-GUARDS | Activate drum/steam/weight in randomized order at valid fixed seeds | No softlocks, no activation loop, fixed upper bound on damage/events | No unavoidable unfair hazard chaining |

**Review targets, not automated "fun" claims:** initial read ≤3 seconds; 2+ genuinely different winning strategies in observed replays; the major chain is optional and consistently reproducible when deliberately arranged; no more than 4 concurrent enemies in the first variant; room time on first human passes tentatively 90–180 seconds. Capture three playthroughs (direct / one-prop / full chain), annotate missed opportunities and fix unreadable hazards before adding content.

## 8. Dependency-aware build path and scope fences

1. **B-00 audit + base room:** reuse actual active scene loader, level material rules, enemy spawning and checkpoint contract. Prove 2P/solo ground path, entry/exit and 2-level readability. No new art or mechanics yet.
2. **B-01 valve:** data-authored trigger, state machine, visible warning, deterministic steam knockback/damage, contact attribution and reset.
3. **B-02 hanging weight:** breakable low tether, timed shadow, deterministic fall/crush, low traversable settled remnant; independent tests.
4. **B-03 collisions/chains:** launched goblin contact with fixture, optional steam-damaged-anchor coupling, bounded real interactions; no bespoke kill script. Do not make direct gameplay depend on unmerged CP-02 enhancements.
5. **B-04 optional drum and polished presentation:** only if B-00..03 have human-visible, repeatable quality; use existing art kit and approved scene identity with original editable asset sources, provenance and source manifests where needed.
6. **B-05 live playtest / evidence:** actual Bevy captured at 1× with approved hero masters, gamepad runs, session traces, no regression, performance and human notes. Fix clarity before extending mechanics.

**Dependency gates:** do not claim upstream dynamics implemented if they are only proposals. If dynamite, enemy-on-prop collision, terrain destruction, camera or companion behavior are unavailable, ship **an honest gated subset** that works using currently merged primitives and report outstanding gates in the PR. Feature gates must never leak late ability loadouts into pre-Hoarder campaign or normal demo. No new HUD inventory or reward system.

## 9. Required sources and precedence

- [Approved combat-first pillar](combat-first-encounter-design.md)
- [Floor 01 progression](floor-01-gameplay-progression-v0.md) and [living horizon](character-progression-horizon.md)
- [MVP controller grammar](mvp-controller-contract.md) and [ADR-0010 no-inventory progression](../05-decisions/ADR-0010-combat-first-progression.md)
- [Goblin first encounter / CP-02 collision baseline](floor-01-first-goblin-encounter.md)
- [First-floor art atlas](../02-art/floor-01-art-atlas-v1.md) and [enemy roster](../02-art/floor-01-enemy-concept-roster.md)
- [Level data and reachable routes](../03-tech/level-data.md) and [asymmetric gates](asymmetric-traversal-and-gates.md)
- [COMBAT-LAB-01 rooms](combat-lab-01-rooms-r00-r02.md) for **existing** playable-test geometry limitations, not a new level loader
- [Playtrace session recording](../03-tech/playtrace-01-session-capture.md), [testing](../03-tech/testing.md), [rights & release](../04-production/rights-and-release.md)
- [Agent implementation work order](../07-agents/floor-01-goblin-boiler-workorder.md)

This is an **original game-design adaptation**, not a claim that the source novel contains a literal boiler chain scene. No novel text/assets or third-party Metal Slug art may be copied.
