# F02-COMPOSE-01 — Encounter composition, difficulty rhythm and player-authored spectacle

**Status:** proposed scene composition and numeric hypotheses, NOT implemented or owner playtest-approved, 2026-11-22. Read the [F02 game bible](floor-02-last-lesson-design-bible.md), [enemy acting](floor-02-enemy-acting-bible.md), [chapter director](floor-02-director-and-continuity.md), [AUTOPLAY design](../03-tech/autoplay-01-automated-gameplay-evaluation.md) and [PLAYTRACE](../03-tech/playtrace-01-session-capture.md). Keep the original controller, the currently validated Floor 01 enemy/prop budgets and the owner's pending Floor 01 game-feel baseline authoritative.

## 0. Increasing difficulty without bloating enemy health

**Floor 01 teaches the verbs; Floor 02 tests decisions using the same verbs.** Difficulty should emerge from two *readable* overlapping intentions, terrain that matters to combat, real enemy-body impacts, bounded time to interrupt a support actor, and genuine recovery from an interrupted plan. It must NOT be an increased-HP ladder, random undodgeable homing, repeated wave filler, spell immunity, three mechanics taught at once or a giant universal HUD.

First introduce each new attack alone, then compose it with exactly one previously understood pressure. A player must retain at least one physically reachable counter using the already-earned abilities of Carl or Donut. Normal direct combat is still a satisfying first-class route. Optional prop improvisation gives speed and showmanship, not a puzzle-only immunity bypass.

**Initial roster budget hypothesis:** 1–3 concurrent offensive actors in ordinary mixed waves. Floor 01 PT-TUNE documents existing max_living=4 and MAX_ROSTER_WAVES=8 in the relevant wave mode; **do not raise those limits** without inspecting the actual schema and measuring a specific need. A boss begins without adds. Grub clusters, projectiles, convoy and independent environmental warnings need separate measured simultaneous-load budgets and cannot be counted as free props.

## 1. Seven combat roles

| Role | Enemy family | What the role changes | Player answer |
| --- | --- | --- | --- |
| RUSHER | Clurichaun runner / mounted kobold | closes distance; stationary play becomes risky | bait committed line, jump, stagger, body launch |
| LINE_CONTROLLER | Mind Horror | a clear directional psychic lane | flank and punish exposed recovery |
| RANGED_PIN | Clurichaun slingshot / Vespa | single reachable projectile lane | reposition, counter, interrupt |
| INTERRUPTIBLE_SUPPORT | Laminak elite | threatens one finite extra enemy on a long alarm | interrupt real landed hit; direct kill |
| BODY_FODDER | clurichaun / dismounted kobold | expressive low-health target to launch | direct attacks or launch into an actual object |
| ECOLOGY | Brindle Grub cluster | bounded corpse activity and altered local geometry | ignore, control, reposition or use conveyor |
| EXCEPTIONAL | Krakaren / Ralph / Rage | stage-specific attack reading rather than more fodder | direct boss recovery or authored Rage flight |

A mounted kobold/dingo pair communicates **one combined charge** until an authoritative dismount; after separation they occupy separate actual budget slots. Kraken tentacles are authored boss attack volumes rather than extra full-DPS enemy agents. Rage has no normal boss HP and no ordinary mob-wave companions.

## 2. Enemy pairings: one tactical interaction each

| Pair or system | Memorable player-created possibility | Counterfeit difficulty / degenerate case to reject |
| --- | --- | --- |
| Mind Horror + rusher | sidestep psychic lane, Carl kicks rusher into service counter, Donut hits exposed brain | safe zone entirely sealed by overlapping aimed attacks |
| Clurichaun + Laminak | one slingshot forces movement, while executive alarm begs to be interrupted | infinite reinforcement or two simultaneous unavoidable shots |
| Low-health seller + jug rack | real launched-body impact spills jugs, second action ignites, falling sign crushes only contacted targets | Donut pressing Magic Missile once auto-completes scene |
| Grub clusters + conveyor | corpses influence moving physical hazards | unlimited enemy proliferation after stylish kills |
| Vespa + one ground foe | Donut attacks aerial line, Carl interrupts foe then punishes landing Vespa | permanent untouchable flying turret |
| Convoy + Vespa | protect humans while relocating across two lanes | offscreen civilian instant death or required precise ally jump |
| Convoy + mounted kobold | knock rider from dingo before charge reaches wheelchair stop | forced two-controller attack on exact tick |
| Kobold + speaker rig | throw detached rider into actual loudspeaker support | scripted speaker auto-fall on entering a trigger |
| Ralph + arena rail | lure clearly committed rush into real impact, punish with either hero | random teleport and oversized invisible damage box |
| Rage + safe stairwell | read monumental committed lunge and use floor rule to exile it | deceptive HP meter, instant loss or accidental floor-3 transition |

**High-pressure scene arithmetic:** one active support alarm + one committed slingshot + one armed explosive already represent three simultaneous player attention demands, even if two are not living enemies. Do not activate a hidden fourth offensive rusher simultaneously. The art can imply more chaos than the authoritative simulation needs to host.

## 3. Encounter roster and dramatic pressure across five chapters

This is **design-only**, not data shipped or measured human clear time.

| Beat | Introductory roster / sequence | Combat shape | Direct response | Optional physical payoff | Pressure |
| --- | --- | --- | --- | --- | --- |
| S00 | one familiar weak foe, scenic cleanup grub | new palette, old controls | normal punch/kick or cat attacks | crack noncritical tile | low |
| S01 first | one Mind Horror | one psychic line and generous recovery | reposition + direct punish | slam into counter | low |
| S01 second | Mind Horror + one rusher, staggered | first two-intention encounter | flank or intercept rusher | drop number sign by true contact | medium |
| S02 opening | slingshot rep + one runner | horizontal ranged plus approach | move, close, attack | body vs display | medium |
| S02 center | rep + runner + Laminak, max one reinforcement | alarm decision plus jug-risk | interrupt + finish by direct blows | 2-step jug-to-sign chain | high, peak 1 |
| S02 end | two known foes | recovery beat | normal direct attacks | no mandatory prop | lower |
| S03 | rooted Krakaren alone | committed slam, sweep or pull one at a time | exploit reachable core windows | capped sign/jug strike | high, peak 2 |
| S04 | finite grub clusters then one Vespa + one melee | moving conveyor and aired hazard | physical/magic and movement | actual bin contact kill | medium |
| S05 stop 1 | one Vespa + one familiar pressure, staggered | protect convoy, upper/lower lanes | direct clear or authored retreat | industrial collision | medium-high |
| S05 stop 2 | one mounted pair + one keeper at most | dismount/protection | direct clear | rider into road sign | high |
| S06 | Rage only, refuge | invulnerable warning then safe flight | visible sanctuary | anticipation of committed charge | high emotion, little clutter |
| S07 | Rage plus ONE sequential lifting hazard | committed pursuit, two safe alternatives | no-cart route to side corridor | decoy-induced exile | final escape climax |
| S08 | mounted rider pair + keeper | dingo charge, local ally choice | direct attacks | speaker / optional kennel rescue | medium-high reset |
| S09 | Ralph ALONE | tiny fast single-threat boss | two-hero direct punish | conditional dog interruption | high precision, uncluttered |
| Mongo room | no enemies | quiet reset | recognize pet, real exit | no score farm | zero |

**Five-chapter pacing:** A first mastery, B two major spectacle peaks separated by a breather, C complex physical world resolving into a human-protection goal, D impossible-threat climax without gratuitous extra mob waves, E funny animal/dark arena followed by actual tiny skill duel and gentle Mongo ending. If first-clear duration becomes padded, merge *beats* without taking away player agency.

## 4. Per-scene data contract (adapt to actual existing schema)

An authored scene needs: unique beat ID, required story grant set, checkpoint, geometry/camera bounds, one entrance silhouette, all threat role IDs and max simultaneous active demands, activation/entry triggers, clearly marked safe lanes across every danger phase, a direct-combat clear condition, a player-authored optional physical route, a missed-prop recovery route, unique causal event IDs, solo follow/escape path, co-op regroup boundary and exact evidence paths. Keep all candidates as data or narrowly scoped tests rather than inventing a second encounter scripting engine.

Source data should distinguish enemy role slots from independent active hazards; an environmental fixture does not become a hidden damage hazard just because its rendered art moves. Never spawn in the hero hurtbox, out of a camera-safe arrival or inside a refuge.

## 5. Timing rules and anti-degeneracy

- Show the first telegraph for a new family while **only one** live attack requires attention. At later mixes, attacks may overlap, but collision threat volumes and legal movement counter must be checked, not assumed.
- Stagger introductions rather than placing three ready-to-fire ranged enemies just offscreen. The opening source of a projectile should be visible or have an honest approach warning.
- Foreman/Laminak alarm may raise **one finite reinforcement**; failing to interrupt is harder but never creates infinite waves or a route softlock.
- Environmental chain danger applies to **both** protagonists, with the actual current friendly-hazard policy and visible fuse. A solo AI partner must avoid the fuse and not ignite it just because a prop is targetable.
- Do not scale difficulty by secretly detecting human vs AI control, granting enemies new immunities or adjusting collision after seeing the input. Fixed scenario contract and replay hashes should remain reproducible.
- Recovery after a big result must be long enough to perceive it; the camera and CINE slow-mo presentation must not cover another incoming tell.
- Donut stationary Magic Missile may win *some* encounters; this is not globally forbidden. In **targeted mixed-pressure S01/S02 probes**, compare stationary vs mobile to ensure repositioning provides a meaningful advantage without making magic useless. A stationary bot dying is not sufficient evidence of fun.

## 6. Causality for spectacular kills

Every signature setpiece is tested four ways:
1. Normal direct combat with all optional props unused.
2. One authentic prop use from a legitimate hit or body contact.
3. Multiple authored decisions producing an attributable multi-link chain.
4. A missed/failed prop setup followed by successful normal combat.

The reward engine receives only factually attributed damage/contact/assist events. Scripted scenery destruction is not a gameplay feat. Rare cinematic deaths must pass existing CINE qualifications; a normal barrel kill, however attractive, must not produce slow motion every time.

**Companion and two human players:** actual contribution is measured, not inferred from being on screen. One character landing a finisher does not zero the other's genuine setup assist; per-character viewer counters stay individual, and no added global scoreboard mode is approved here.

## 7. Implementation gates and owner playtest

- **CMP-01:** no novel two-role encounter before the primary new role has an isolated successful readable test.
- **CMP-02:** every mixed-threat set has at least one demonstrated safe counter and real direct route for each lead hero.
- **CMP-03:** no raise of existing enemy-wave caps or hidden extra active projectile budget.
- **CMP-04:** no two warning volumes produce an unavoidable full screen damage trap.
- **CMP-05:** no destruction of the only exit or permanent safe lane; repeat with all optional props broken.
- **CMP-06:** each signature room supports direct, one-prop, multi-link and missed-setup pilot (where meaningful).
- **CMP-07:** reproduction captures active slots, hazard overlap, contact causes, viewer attribution, camera visibility and no-progress spans.
- **CMP-08:** fixed-seed retries reset threats, props, story grants, scores and exit state.
- **CMP-09:** Donut-leading, Carl-leading and AI-companion evaluations report actual attacks/hits/evades; 2P human feel PENDING.
- **CMP-10:** human verdict for hit feel, humor, discovery, pressure and fun is NOT RUN until owner plays.

**Agent prioritization:** first validate S01 Mind Horror plus rusher, S02 rep plus Laminak plus prop *without* raising caps, and S03 rooted Krakaren directly. Only then scale to conveyor/convoy/kennel; Rage/party chase may remain independent pure simulation if camera or companion baseline is unresolved. The next document defines the evaluation experiments and false-confidence protections.
