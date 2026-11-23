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

## 10. Room 2 retune: FA-Q31 (deferred), FA-Q32 to FA-Q37 (adopted)

The owner played the live room and found it "tooooo easy": the AI Donut did the killing. A read-only analysis of the recorded session (one human-held Carl, one AI Donut) found the cause in the partner's kill share (her missiles out-range goblins and she fires at any unclaimed enemy, awake or not) and, second, in the thin enemy pressure (the wake columns 124, 420, 560 and 560 plus 90 made almost every fight 1v1). Enemy hit points stay as they are (goblin 2, Engineer 3): the canon forbids hit point sponges. Every value below is data (the sidecar, or `assets/data/gameplay/f01_boiler_fixes_v0.json`), the owner may change any of them in a follow-up PR, and the measured result is in `docs/evidence/f01-playtest/README.md`, section 5.

| Id | Value | Why |
| --- | --- | --- |
| FA-Q32 | sidecar `g2.trigger_x` 224 (it had none), `g2.delay_ticks` 15 (it was 120) | g2 wakes when a hero leaves the protected spawn (x 16 to 224) plus 15 ticks, so g1 and g2 are B1's staggered pair. g2 stays at x 640 beside the steam lane, so B2's venting still catches it. The first value tried was 30 ticks; the retune's targets needed 15 (iteration 1, below). |
| FA-Q33 | sidecar `e1.delay_ticks` 15 (it was 90) | B3 arrives as a pair (g3 at tick 0, the Engineer 15 ticks later), not as two 1v1s. The stagger rule still holds (same trigger column, different delay). |
| FA-Q34 | `fa_q34_finish_reserve_px` 200 (absent = off) | An enemy with at most one hit point within 200 px of the human-held hero is his to finish. It joins the leader's claim set (so Hunt never picks it and it is never a steal) and the AI withholds any press she would start against it, but fires no 0-damage glance (a glance every 16 ticks would stun-lock it). The one exception: its attack tell is aimed at her, and then a lethal hit is a glance. Beyond 200 px she may kill it (owner decision 0.1(1): "if Carl is far we should be able to kill them autonomously"). Boiler only. |
| FA-Q35 | `fa_q35_hold_fire_until_awake` true (absent = off) | No press targets an enemy that is not awake: B0 "read the machine" is the human's. "Awake" is a persistent per-enemy flag, set when a hero has ever been inside its sight, when it was touched, or when its wake gate released and the delay elapsed (the first version used the wake gate alone, which g1 does not have). Self-defence against an awake attacker is untouched. Boiler only. |
| FA-Q36 | `fa_q36_wake_leash` true, `fa_q36_leash_margin_px` 4 | Review of the first version of this retune found that the AI hero, not the human, opened the room: Donut led to about x 190 and her own sight woke g1 (it has no gate), so her missiles killed it on tick 68, exactly the owner's opening. While an enemy is not woken the AI hero's movement (follow, hunt, autonomy, nav) is clamped: she keeps at least 4 px outside its sight (220) unless the human leader is already inside it or she is attacked (a hurt, or a tell aimed at her). Knockback and push are not clamped. The human therefore wakes every enemy and B0 is human-paced. A larger reserve radius or a g1 `trigger_x` cannot do this: any hero's sight wakes a goblin. |
| FA-Q37 | `fa_q37_release_on_hits` 1, `fa_q37_release_after_ticks` 150 | The reserve of FA-Q34 had no release: a human who does not finish (a slow pilot) was beaten while Donut watched (29 to 33 hurts and 6 party defeats in the review's probe). A reserved enemy is released when it lands that many hits on any hero, or has been reserved that many ticks in a row, permanently for the attempt (a restart resets it). After a release the AI hero drops her goal plan so that she re-decides on that tick (an implementation clarification: without it she needed 80 to 130 ticks to re-engage and a slow human still lost 5 of 9 runs). Each release is logged with its cause (`reserve_released_hit`, `reserve_released_timeout`, Boiler-only M16). |

**Tuning record.** First version (g2 delay 30, e1 delay 30, reserve 160): the direct route stayed too easy (median exit 505 / 488 / 613). Iteration 1 (g2 delay 15, e1 delay 15, reserve 200) gave 862 / 842 / 862 and worst hurts 3, but the review found two blocking defects (B1: Donut still killed g1 on tick 68 with an idle human; B2: a slow human lost). The ruling added FA-Q36, FA-Q37 and the widened FA-Q35 with their values as ruled (leash margin 4, release after 1 hit or 150 ticks); no tuning iteration was needed for them: the probes P1 to P3 pass at the ruled values, and a sweep of every ruled combination (release after 120, 150, 180 ticks, 1 or 2 hits, margin 0, 4, 16) left the companion's direct route unchanged (median 611) and only `release_after_ticks` 180 with 2 hits lost a slow-human run. **Recorded miss:** with the leash the companion direct median exit is 611 (seeds 520 to 882), under the spec's 650 to 1050 window; the other direct rows hold (862 and 842, worst hurts 3). The g2 and e1 delays move the room chaotically (g2 delay 10, 20, 45 or 60 give a CarlSolo direct median near 500 and 0 hurts), so they were left at 15.

**FA-Q31 (B3 cannot reach the lane or the weight while the drum stands): deferred, still open.** Raised by the PR-C review. The drum's solid (x 848 to 880) stands between B3 (g3 at 944, e1 at 1000) and both the steam lane (688 to 768) and the weight footprint (784 to 848), so a woken B3 walks to the drum's face (x 892) and waits. The retune's spec chose option 2 (a non-solid drum for every actor), but that needs engine code, not data: `EnvRuntime::new` (`src/combat_scene/env.rs`) refuses a drum without `solid_intact` (BOIL-FU), and a launched body strikes the drum through that solid. It is deferred as an owner question (a hero-only solid, which also needs engine code, is the alternative). Until decided, vent-on-B3 stays reachable only after a launched body has blown the drum (test `boil_b3_vented_in_the_lane`), and the B3 line of section 3 ("readable angle to the anchor/weight") stays weak.

**Owner questions** (also in the playtest packet, section 4):

1. Is Donut's "softens but leaves you the kill nearby" (radius 200 px) the right feel, and should FA-Q34 and FA-Q35 later apply to every room, not only the Boiler?
2. The drum: is it acceptable to wait for an engine change for FA-Q31 (a non-solid or hero-only drum), or does B3 at the drum's face read fine as it is?
3. Is a direct-route clear with up to 3 hurts the right difficulty, or should the room add a fifth enemy under `waves: true` (opt-in, PTM-AD6)?
4. Is it right that nobody fires at a sleeping goblin, or should Donut be allowed a deliberate opening shot on the human's input only?
5. With an idle human the room stays asleep until Carl moves (Donut keeps out of every sleeper's sight). Is that right for B0?
6. Should a reserved enemy be released after 1 hit on a hero (adopted) or after 2?
