# F01-CHASE-DIRECTOR-01: Murder Dozer opening, playable cinematic choreography

**Status:** encounter direction and an agent implementation specification. It is **not implemented and not playtest-approved**. It enhances the approved ENV-02 route **without replacing its geometry, asset contracts or controller grammar**. The existing Floor 01 design authorities still govern narrative and ability decisions.

**Goal:** a player should think "we barely survived that thing", and should remember it when the later, optional Dozer revenge in room 3 comes. Fear and scale come from **real-time play**, not from a noninteractive cutscene.

**Provenance.** This document comes from the owner's draft PR #179 (F01-CHASE-DIRECTOR-01). The draft PR stays open and unmerged as the record of the original text. This copy keeps the draft's direction, phases, QA table and playtest questions. It adapts the parts that main has since overtaken, as listed below. Where this document and [the Act I integration plan](floor-01-act-01-integration.md) disagree, the integration plan holds the numbers and the contracts; this document holds the intent.

**Adaptations to main (ACT1-PLAN, 2026-11-25):**

1. **The machine exists.** Room 3's DOZER-CORE machine (`src/dozer.rs`, `src/combat_scene/dozer.rs`, the DOZ-01 rig at h120) is merged. The opening reuses it with a second profile `opening` and does not build a second vehicle (integration plan section 5).
2. **Speeds.** The draft's "cruise about 130 px/s against Carl about 145 px/s" were early ENV-02 hypotheses. On main Carl runs 185 px/s. The opening uses DOZ-01 section 9's values instead: cruise 160, catch-up 185, mercy 150, a 36-tick tell, then a 240 px/s charge of at most 90 ticks, overshoot brake 24 ticks and strain 90 ticks.
3. **Fairness.** The draft's preferred policy 1 (arm on the leader, commit on both) is adopted as a measured commit rule (integration plan section 5.3). The rule also requires that the charge reaches the barricade, so the wheel always passes the alcove and then breaks the barricade in one committed run; after the impact the charge runs on to the wedge instead of overshooting (ACT1-AD18).
   - If the rule never holds, the machine does not charge: the cruise still breaks the barricade.
   - The draft kept the 8 s fail-open timer as a last resort. It is no longer needed, because nothing can softlock and nothing forces a lethal charge.
4. **Rooms.** The `--f01-run` floor run loads one room per launch, and a respawn resets the machine. The opening is therefore two run rooms that meet at `checkpoint_post_chase` (route x 2640):
   - `act1_chase` holds C0 to C7;
   - `act1_shore` holds C8.

   A retry after the chase can never rerun it (CHDIR-09). The cost is a room-clear panel between C7 and C8 (ACT1-AD2, ACT1-Q1).
5. **Audio.** Both provisional files exist: `assets/audio/floor01/music/floor01_dozer_chase.ogg` and `assets/audio/floor01/sfx/murder_dozer_engine_sketch.ogg`. The game has **no runtime audio path** yet, so section 5's audio direction is deferred (ACT1-Q2). It stays here as the brief for when audio lands.
6. **Riders.** The greybox opening machine has no riders. The shore goblins stand in for the thrown riders (ACT1-AD9, ACT1-Q6).
7. **Superseded coordination.** The draft's conflicts list is resolved. #174, #175, #176 and #178 are merged, and the room 3 revenge is built from #178.

## 1. Non-negotiable source of truth

Read before implementing:

- [ENV-02 opening geometry](../02-art/env-02-opening-spec.md): **option R**, segments S1 to S6, the exact chase markers, the safe pocket with 104 px clear, the charge and barricade, and the wedge. This document does not authorize new topology. `assets/levels/floor01/opening_route_proposal.json` is the source layout proposal; do not replace it with a competing level. The run rooms copy its x values (integration plan section 4).
- [DOZ-01 native art, rig and timing](../02-art/doz-01-dozer-spec.md):
  - the approved h120 machine scale: 259 px long, **155 px tall including the stack**;
  - a lethal front wheel;
  - a 4-frame spinner, 9-frame treads and riders.

  The opening Dozer is an **escape threat, not an HP-bar boss**. The source art is in `assets/source/enemies/dozer/`, and room 3 already draws it.
- [The Dozer gameplay note](enemy-gameplay-dozer.md), section 4.4: what the opening gets from DOZER-CORE.
- [Act I beats](floor-01-act-01-playable-combat-beats.md) and [the first goblin fight](floor-01-first-goblin-encounter.md): the I-03 retaliation comes immediately **after** the escape. Preserve the first-goblin design and `checkpoint_post_chase`.
- [Controller grammar](mvp-controller-contract.md), [character progression](character-progression-horizon.md), [camera contract](../03-tech/input-camera.md), [Playtrace](../03-tech/playtrace-01-session-capture.md), [rights](../04-production/rights-and-release.md). Carl and Donut start controllable in 2P. Before the Guild there is no dynamite, no Magic Missile, no dodge unlock and no Puddle Jumper.

**Explicit scope:** the choreography, staging, audio and visual emphasis, fair 1P/2P chase timing, environmental response, replay and tests of **one** introductory pursuit. Do not redesign the later optional Dozer revenge (room 3, merged from design PR #178) or the Goblin Boiler (merged from #176).

## 2. Player-facing experience and pacing

Target a **first-time entry-to-survival traversal of roughly 25 to 40 seconds**, not counting death retries or pauses. An experienced player should clear it noticeably faster.

- This is an *initial rhythm hypothesis*. It is not a cutscene timer and not an excuse for artificially slow movement.
- With the route x 72 → 2640 and the current movement, tune against real timed player traces (ACT1-Q9).
- Keep the **active S3 to S4 chase brief, escalating and replay-friendly**, tentatively about 10 to 18 s of meaningful running and safety choices.

| Phase | Segment / markers | What the player sees and hears | What the player DOES | Critical read |
| --- | --- | --- | --- | --- |
| **C0, false calm** | **S1** x 0 to 640, spawn x 72 | Warm sparse torches, deep arches, a distant mechanical tremor; the whole machine is not shown yet | Move and jump freely; learn normal rightward travel | The quiet is deliberate; **do not** add a forced tutorial |
| **C1, something is coming** | **S2** x 640 to 944, the window | A chain trembles or a low mechanical knock sounds from behind; firebox glint and smoke in the window; then the spiked wheel | Keep moving; look back if they choose | An on-screen or audible **advance warning** before any lethal pursuit |
| **C2, the machine enters** | `checkpoint_chase` x 1008, `trigger_chase_start` x 1072 | The Dozer comes out of the S2 arch with its authored turn-in (about 0.8 s); the engine and wheel sound grows | Run; no need to attack | Show its h120 / 259 px mass and the wheel silhouette without freezing inputs |
| **C3, it destroys the route** | **S3** x 944 to 1712, option R lowered ceiling | The two authored timber sets splinter **as the Dozer's front actually passes**; two one-tile humps go under the wheels; brief masonry dust and kettle rattling | Jump the low humps, stay ahead, see that timber can break; no fake mandatory melee | Distinct diegetic destruction. The Dozer **must not** crush a player unseen from beyond the camera |
| **C4, a commitment, not homing** | **S4**, charge arm x 1888 | The engineer pulls a lever, a whistle and click, the wheel visibly speeds up, the engine pitch shifts before the committed charge | Spot the lit recessed pocket ahead and choose to hide | The tell starts **before** the fast lethal closure; direction and duration are readable |
| **C5, the narrow escape** | Alcove ledge x 2064 to 2160, 104 px clear inside | Both heroes enter the pocket; the wheel crosses in the foreground in a huge near miss; brief sound occlusion under the arch | One small hop into an 80 px wide shared refuge; wait briefly | The refuge fits **both** Carl and Donut; no input lock; no forced 1P swap gate |
| **C6, barricade impact** | Full-height timber barrier x 2304 to 2400 | The Dozer hits the planks and breaks them in a physical impact beat; splinters scatter; riders pitch forward | Wait safely; see the obstruction gone | The front's collision removes the **real blocking collider in the same authoritative tick** |
| **C7, overshoot and aftermath** | S5 entry x 2608, the wedge, `checkpoint_post_chase` x 2640 | The machine cannot turn and wedges into the S5 arch; the stuck wheel spins; the kettle wheezes; the space opens up and goes quiet | Leave the pocket, run forward, get ready to hit goblins | No long noninteractive victory scene. The Dozer stays alive as stuck machinery and is **not killable here** |
| **C8, immediate retaliation** | S5, the first goblin encounter (`act1_shore`) | One goblin's attack tell; the first direct kill; later the rock-shore opportunity | Punch, kick, and use the CP-03 stone if desired | **The first ordinary kill** satisfies in contrast with the Dozer's invulnerability |

These are *dramatic beats*, not eight scripted cutscenes. Real authored marker and collision facts trigger each beat. No scripted player displacement and no camera-led death. One standout beat is enough: **the wheel passing inches in front of both heroes in the alcove, then demolishing the barricade**.

## 3. Preserve the exact ENV-02 spatial interface

| Marker | Baseline | Required interpretation | Run room (integration plan section 4) |
| --- | --- | --- | --- |
| S1 / S2 / S3 / S4 / S5 / S6 | x 0 to 640 / 640 to 944 / 944 to 1712 / 1712 to 2608 / 2608 to 3296 / 3296 to 3584 | Do not change the segment order or add a second chase corridor | S1 to the S5 arch in `act1_chase`; S5 to S6 in `act1_shore` |
| Chase start | x 1072 | Start the real pursuit once, after the scene or retry has set up capabilities | Trigger crossed by the rearmost hero (ACT1-AD6) |
| S3 lower ceiling | **option R**, about 212 px clear | The Dozer is 155 px tall; keep its mechanical silhouette and no ceiling penetration | Decor, not a solid (ACT1-AD8) |
| Charge arm | x 1888, minimum gap about 140 px | The Dozer must show a **committed forward charge**; it never turns or reacquires a player mid-charge | Arm on the leader, commit rule on every hero |
| Shared refuge | x 2064 to 2160, raised 16 px, **104 px clear**, 80 px usable | Fits both heroes; safety comes from the real refuge geometry | Pocket `{2072, 460, 80, 104}` on the ledge |
| Barricade | x 2304 to 2400, the full-height prop variant as the gameplay blocker | Only the Dozer clears it in the opening; no jump-over, no early destructible exploit | Lock `barricade`, `opens_on: dozer_contact` |
| Wedge | S5 arch, x 2608 onward | Needs an **end-on / wedged pose** at most 192 px wide (or ENV-02's wall-breach fallback); do not squeeze the 259 px side-on rig through the 192 px window | `stop_x` 2856, greybox wedge draw, art is ACT1-Q3 |
| Checkpoints | x 1008 (chase), x 2640 (post-chase) | Fast retry near the danger; a retry after the chase never reruns the chase | 1008 in `act1_chase`; 2640 is `act1_shore`'s entry |

Authored tests checked the ENV-02 reserve geometry. This spec requires proof in the *real Bevy scene, physics, camera and local co-op*, not an inference from the key sheet.

## 4. Chase control and fail-safes

### Real motion, no fake cinematic threat

- **Speeds.** The values are DOZ-01 section 9's (adaptation 2): cruise 160, catch-up 185, mercy 150, a 36-tick tell, a 240 px/s charge of at most 90 ticks. DOZ-01's wheel and tread motion runs on the fixed tick and distance. These are **starting points**, not accepted balance. Derive the real elapsed time and gap in a deterministic simulation and tune against human inputs.
- **Lethal contact.** The front wheel is the clear lethal contact zone; keep DOZ-01's safe-jump constraints. Do **not** make "jump over the Dozer" an alternative: the approved jump apex is below the wheel's top.
- **S3 timbers** are a one-way **presentation response to the real machine-front crossing**, not a new physics simulation. Splinter effects never cause damage. The humps stay real simple traversal surfaces; the machine visibly flattens them without killing a player outside its contact.
- **Cruise speed** never silently speeds up because the player catches up with the camera. The charge starts with a detectable tell (lever, whistle, wheel smear) and then commits. No hidden position snapping, no instant adaptive pursuit, no homing into a hiding player.

### Solo and 2P: the real risk

**The original "leading hero crosses x 1888" trigger can be unfair in local 2P when the second hero is hundreds of pixels behind.** A fast player could arm the charge while P2 cannot physically reach the x 2064 pocket within the charge. This is a **design problem to solve**, not something to hide behind the camera leash.

Treat the x 1888 marker as the **arm**. Before the full lethal charge starts, prove that every living hero can reach a safe pocket from its real position at its measured speed. The draft compared three policies:

1. **Preferred experiment, adopted:** the visible windup begins when the leader arms. The charge commits only when **every** living controllable hero is in the reachable preparation band. If the rear hero is late, the pursuit stays fair and readable; nobody is invisibly teleported. Integration plan section 5.3 states the measured rule.
2. **Fallback, partly adopted:** keep the anticipation, the measured gap and the pursuer speed inside the same ENV-02 geometry, and keep the wheel near miss.
   - The draft kept the 8 s `barricade` fail-open as a last resort. Main replaces it: with no charge, the cruise reaches and opens the barricade, so no timer is needed.
   - The draft's limit stands: nothing may ever secretly force an unavoidable lethal charge on a straggler.
3. If no configuration fits extreme 2P separation, flag the geometry or camera conflict for owner review. Do not silently change core 2P camera rules, resize the niche or give one player fake invulnerability.

**Mandatory cases:**

- two real pads, both entering the refuge;
- P2 0, 96, 224 and 448 px behind at the arm;
- reversed leader identity, if party ownership supports it;
- one hero reaching the niche first;
- solo Carl-active and Donut-active with the AI follower, with correct recovery.

The AI follower is helped by DOZER-CORE's existing rescue, plus two appended spots, Pocket and Behind (integration plan section 5.5). No new follower navigation or teleport mechanics are added. An unavoidable 2P death from a valid starting separation is a **release blocker**.

### Safety and difficulty

- A chase hit uses the existing health and retry rules: the wheel defeats as in room 3. **Never** kill because the renderer thinks the Dozer looks close; collision is authoritative.
- The default chase must be doable on the first run without pixel-perfect jumps or secret interaction buttons. The x 2064 niche must read as safety before the wheel commits.
- After a failure, an **optional** contextual hint ("GET INTO THE ALCOVE") may appear at the checkpoint, and only after one failure. Use an accessibility setting if one exists. Do not make a big arrow or an intrusive scripted prompt mandatory on a fresh run.
- Retries **restore the destructible props, the barricade collision** and the Dozer's deterministic motion, and let the player resume at x 1008 without waiting through the quiet intro.

## 5. Camera, audio and VFX direction (presentation only)

### Camera

- Keep the 640×360 and 640×400 native pixel presentation and the existing integer snap and smoothing. Show the machine **in frame well before contact**, without permanently zooming out and without cinematic black bars.
- In S3, prefer stable **forward-travel framing** with enough rearward context to see the wheel closing. Showing only part of the 259 px body is acceptable when needed; an offscreen lethal front is never acceptable.
- At x 1888, reveal enough **forward** sightline to recognize the niche. Then hold a composition that shows (a) the players inside, (b) the wheel passing and (c) the barricade ahead. Do not cut to a camera angle that removes agency. A small presentation-only shake at the barrier impact is acceptable if it never changes hit boxes or fixed-tick state.
- In 2P, actor separation and the stop / overshoot camera clamps are harder than in solo. Record 640×360 and 640×400 captures at the peak of the charge and at the barricade crash. Any camera-rule change outside the existing ADR and contract needs separate approval.

### Audio (deferred: no runtime audio path, ACT1-Q2)

The repo contains `assets/audio/floor01/music/floor01_dozer_chase.ogg` and `assets/audio/floor01/sfx/murder_dozer_engine_sketch.ogg`. Both are provisional starting material, not accepted final mixes, and the game loads no audio today. When an audio path exists:

- **C0 / C1:** a low mechanical vibration through otherwise sparse catacomb ambience. Separate it from ordinary combat with **negative space**.
- **C2:** a clear metal-wheel and engine entry stinger. The chase music starts **on the gameplay trigger**, never on a timeout from scene start.
- **C3:** two distinct, spatially grounded timber breaks and a rising engine pitch. No permanent continuous wall of noise.
- **C4:** a metallic lever click, a whistle and a rising tread cadence. It must stay recognizable with reduced VFX.
- **C5 / C6:** a fast near-field wheel pass, one loud **brief** timber impact, then a falling engine and wheel strain. Ducking protects gameplay tells and avoids constant loudness.
- **C7 / C8:** the music **cuts or resolves** into relative quiet. Then the first goblin hit lands as a strong, dry contrasting impact. An audible "we lived" moment beats an extra cinematic scene.

All audio sync uses authoritative events. Music, camera and particles never change simulation state, spawns, damage or fixed-step timing.

### VFX and art

- The first pass needs only the authored timber smash props, the DOZ-01 wheel, treads and smoke, and the agreed opening markers.
- Splinters, shake and crumbs are **presentation**, capped, and layered correctly front and back. Show the wheel's bright spike ring against the brick.
- The end-on / wedged silhouette has an art gate (ACT1-Q3). Do not stretch the 259 px side-on rig through the 192 px arch, and do not recolor or replace approved master sprites.
- **No slow-motion reward** for merely surviving the chase. Cinematic slow motion is reserved for exceptional skilled kills (CINE-01); the first victory comes later, in the goblin fight.
- Do not invent weapons, magic, an audience, viewers or sponsor gifts in the opening.

## 6. State machine and event contract

The draft suggested **narrow chase phases**: `Dormant` → `EntryTell` → `Cruise` → `ChargeArmed` → `ChargeTell` → `ChargeCommitted` → `BarricadeImpact` → `Overshoot` → `Wedged` → `Resolved`. It asked to collapse them where the engine has a simpler equivalent and to avoid a general vehicle AI framework.

**On main** they collapse onto DOZER-CORE (integration plan section 5.2):

| Draft phase | Main state |
| --- | --- |
| `Dormant` | `Dormant` |
| `EntryTell` | `Approaching` (rumble) |
| `Cruise` | `CommittedChase` with its speed band |
| `ChargeArmed` | an event |
| `ChargeTell`, `ChargeCommitted`, `Overshoot` | sub-states of `CommittedChase`, present only in the `opening` profile |
| `BarricadeImpact` | an event plus the `dozer_contact` lock |
| `Wedged` / `Resolved` | `Escaped(Idle)` with the wedge flag |

The draft's semantic events map to appended `DozerEvent` codes 13 to 20:

| Draft event | Main |
| --- | --- |
| `ChaseStarted` | the existing `Rumble` / `Emerged` |
| `TimberSmashed(id)` | `TimberSmashed { id }` |
| `ChargeArmed` | `ChargeArmed` |
| `ChargeCommitted` | `ChargeTell` and `ChargeCommitted` |
| `PocketEntered(hero)` | `PocketEntered { hero }` |
| `BarricadeDestroyed` | `BarrierImpact` |
| `DozerWedged` | `Wedged` |
| `ChaseResolved`, `CheckpointAdvanced` | the room exit and the run's next room |

The rules:

- Each timber hit is one-shot per marker id.
- The barricade is removed **in the same sim tick** it is hit.
- The wheel hazard turns off at the wedge.
- Actor ownership and the defeat and retry events stay as they are.

Every gameplay state that can affect collisions, spawns or progression is in the deterministic state hash and in the journal and snapshot contract; presentation-only VFX and audio are not. Repeated retries reproduce the same authoritative transitions under the same inputs and ruleset. The chase runs through the current run-room integration (`--scene f01_late --f01-run`), never through a duplicate scene framework.

## 7. Required real-gameplay QA

The ACT1 column names the headless fixture or test from [the integration plan section 10](floor-01-act-01-integration.md#10-tests-and-fixtures-by-id).

| ID | Scenario | Headless criteria | Human / native Bevy evidence | ACT1 |
| --- | --- | --- | --- | --- |
| CHDIR-01 | Uninterrupted solo first run | Trigger → timber → charge → pocket → barrier → wedge → checkpoint, deterministic | The player understands the danger and the refuge without a tutorial | F10, F10b |
| CHDIR-02 | Carl tries to damage or clear the Dozer | No vehicle HP drop; no jump through the wheel | Invulnerability shown by sparks, impact and continued motion | F11 |
| CHDIR-03 | The two S3 humps and both timber sets | Timed movement; one break per authored id | Two distinct physical destruction beats, not an invisible instant path clear | F10 |
| CHDIR-04 | The S4 committed charge | The tell precedes the fast wheel; no homing into the pocket | Charge direction and the safe pocket are understood before the reaction deadline | F10, T11 |
| CHDIR-05 | P1 and P2 arrive together | Both fit the niche safely; no accidental collision or shield errors | Both silhouettes legible; the near miss is exciting | F12 |
| CHDIR-06 | P2 behind by 96, 224 and 448 px | A reachable refuge; no **unavoidable** charged-wheel collision in otherwise legal play | Real two-pad camera and recovery | F13, F15, T11 |
| CHDIR-07 | One player skips the pocket; a late retry | Fair failure and checkpoint; no griefing, no softlock; the optional hint after the failure | Failure teaches rather than feels arbitrary | F14, F16 |
| CHDIR-08 | Barrier hit and wedge | Blocker removal in the impact tick (heroes collide with the open state from the next tick, integration plan section 5.4); a single impact; safe past x 2640 | A big smash, then sudden relief and straight into the first fight | F10 |
| CHDIR-09 | Checkpoint and reset at x 1008 and x 2640 | Timbers, barricade and machine back to their start state after a chase retry; no chase after a post-chase retry | Fast retry; the long intro skipped | F14, ACT1-F04 |
| CHDIR-10 | Both virtual resolutions and output scaling | No threat or warning cropped; no invalid zoom; no silhouette distortion | At speed at 640×360, 640×400 and 1080p / Deck-friendly | evidence |
| CHDIR-11 | Playtrace capture and verification | The whole live attempt and its retries captured; the same events and state hashes on replay | Tick-aligned render evidence if supported | T12, PLAYTRACE verify |
| CHDIR-12 | Audio on/off and reduced VFX compared | The same authoritative state, hash and outcome | Threat cues stay accessible without full effects | F17 (audio half deferred) |

**Playtest questions:**

- Did they recognize the machine before it appeared?
- Did they know what to do?
- Was hide → near miss → wood smash a real highlight?
- Was the chase frightening without feeling cheap?
- Did the first goblin kill feel like an emotional release?

Compare **three runs per mode** (first clear, retry, practiced) before freezing timings. No auto-player or ML success score is a human "fun" sign-off.

## 8. Implementation phases and handoff

The draft's phases, mapped onto the two ACT1 PRs:

| Draft phase | Content | On main |
| --- | --- | --- |
| **CH-A, honest base** | Audit the Dozer art integration, the route proposal, the scene entry, the party, the camera and the checkpoints; expose a playable S1 to S5 route | ACT1-A builds the run plumbing and the S5 rooms; ACT1-B builds the S1 to S5 room |
| **CH-B, fixed-step chase** | Spawn and turn-in, wheel hitbox, cruise and the S3 break events, two humps, retry | ACT1-B (DOZER-CORE already holds the wheel, the cruise and the retry reset) |
| **CH-C, spectacle core** | The S4 charge and tell, the pocket and two-player fairness, the barricade impact, the overshoot and wedge, the goblin encounter right after | ACT1-B; the goblins are ACT1-A's `act1_shore` |
| **CH-D, sensory direction** | The native sprite rig and prop variants, restrained audio, VFX and camera punctuation, accessibility and graphics variants | ACT1-B for the rig and splinters; audio deferred (ACT1-Q2); end-on art ACT1-Q3 |
| **CH-E, evidence and review** | Deterministic fixtures, live Playtrace verification, real two-pad footage, measured durations, a log of tuning hypotheses, independent review and owner approval | Both PRs; the owner's game-feel review after merge |

**Out of scope:** the Dozer revenge (room 3, merged), new enemies, bomb, gauntlet or Donut ability work, companion navigation, a new camera architecture, a new cinematic fatality system, special loot or HUD, a general mutable destruction engine and the full Floor 01 route.

**Local agent:** [dedicated work order](../07-agents/floor-01-dozer-opening-workorder.md). No feature counts as implemented merely because this design document is merged.
