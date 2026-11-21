# F01-CHASE-DIRECTOR-01 — Murder Dozer opening: playable cinematic choreography

**Status:** encounter-direction and agent implementation specification, **not implemented or playtest-approved**. This document enhances the approved ENV-02 route **without replacing its geometry, asset contracts or controller grammar**. Narrative/ability decisions remain governed by the existing Floor 01 design authorities.  
**Goal:** make a player think "we barely survived that thing" and remember it when a later, optional same-family Dozer counterstrike is possible. Fear and scale are produced through **real-time play**, not a noninteractive cutscene.

## 1. Non-negotiable source-of-truth

Read before implementing:

- [ENV-02 opening geometry](../02-art/env-02-opening-spec.md): **option R**, segments S1–S6, exact chase markers, 104px-clear safe pocket, charge/barricade and wedge. This document does not authorize new topology. Existing `assets/levels/floor01/opening_route_proposal.json` is the source layout proposal; do not replace it with a competing level.
- [DOZ-01 native art/rig/timing](../02-art/doz-01-dozer-spec.md): h120 approved machine scale, 259px machine length, **155px visual height including stack**, lethal front wheel, 4-frame spinner + 9-frame treads + riders. Opening Dozer remains an **escape threat, not an HP-bar boss**. Existing asset source is in `assets/source/enemies/dozer/`; verify render integration rather than assuming it exists.
- [Act I beats](floor-01-act-01-playable-combat-beats.md) and [first goblin fight](floor-01-first-goblin-encounter.md): immediate I-03 retaliation **after** escape. Preserve first-goblin design and `checkpoint_post_chase`.
- [Controller grammar](mvp-controller-contract.md), [character progression](character-progression-horizon.md), [camera contract](../03-tech/input-camera.md), [Playtrace](../03-tech/playtrace-01-session-capture.md), [rights](../04-production/rights-and-release.md). Carl and Donut start controllable in 2P; no dynamite, Magic Missile, dodge-unlock or Puddle Jumper before the Guild.

**Explicit scope:** choreography, staging, audio/visual emphasis, fair 1P/2P chase timing, environmental response, replay and tests of **one** approved introductory pursuit. Do not redesign the separate later optional Dozer revenge encounter from [design PR #178](https://github.com/pando85/dungeon-floorbreak/pull/178) or the Goblin Boiler work in [#176](https://github.com/pando85/dungeon-floorbreak/pull/176).

## 2. Player-facing experience and pacing

Target the **first-time complete entry-to-survival traversal at roughly 25–40 seconds**, excluding death retries and optional pauses; an experienced player should clear materially faster. This is an *initial rhythm hypothesis*, not a cutscene timer or an excuse for artificially slow movement. With current route x72→2640 and existing movement, tune against actual timed player traces. The **active S3–S4 chase should stay brief, escalating and replay-friendly**, tentatively around 10–18s of meaningful running/safety choices.

| Phase | Authored segment / existing markers | What the player sees/hears | What the player DOES | Critical read |
| --- | --- | --- | --- | --- |
| **C0 — False calm** | **S1** x0–640, spawn x72 | Warm sparse torches, deep arches, distant mechanical tremor; do not show the whole machine immediately | Move/jump freely; learn normal rightward travel | Quiet is deliberate; **do not** add a forced tutorial |
| **C1 — Something is coming** | **S2** x640–944, window | A chain trembles / low mechanical knock from behind, firebox glint/smoke in the window, then the unmistakable spiked wheel | Keep moving; perhaps look back if they choose | Show an on-screen or audible **advance warning** before lethal pursuit |
| **C2 — The machine enters** | `checkpoint_chase` x1008, `trigger_chase_start` x1072 | Dozer comes out of the S2 arch with its authored ~0.8s visual turn-in; engine/wheel sound grows | Run; no need to attack | Reveal its h120 / 259px mass and wheel silhouette, without freezing inputs |
| **C3 — It destroys the route** | **S3** x944–1712, option R lowered ceiling | Two already-authored timber sets splinter **as the Dozer's front actually passes**, two 1-tile humps go under the wheels, brief masonry dust and kettle rattling | Jump low humps, keep ahead, read that timber can be broken; no fake mandatory melee | Distinct diegetic destruction. Dozer **must not** crush the player unseen from beyond camera |
| **C4 — A commitment, not homing** | **S4**, charge arm x1888 | Engineer pulls lever, whistle/click, wheel visibly accelerates, engine pitch shifts before the committed charge | Spot the recessed lit pocket ahead, choose to hide | Telegraph starts **before** fast lethal closure; direction/duration readable |
| **C5 — The narrow escape** | Alcove ledge x2064–2160; 104px interior clearance | Two protagonists enter the pocket; machine wheel crosses in foreground with massive near miss; brief sound occlusion under the arch | One small hop into an 80px-wide shared refuge; wait briefly | Refuge fits **both** Carl/Donut, no input lock, no compulsory 1P swap gate |
| **C6 — Barricade impact** | Full-height timber barrier x2304–2400 | Dozer hits planks, breaks them in a physical impact beat, splinters scatter, riders pitch forward | Wait safely; see the obstruction removed | Barrel/front collision removes **real blocking collider in the same authoritative tick** |
| **C7 — Overshoot / aftermath** | S5 entry x2608, wedge and `checkpoint_post_chase` x2640 | Machine cannot turn and wedges into the S5 arch, stuck wheel spins, kettle wheezes, the background suddenly opens and gets quieter; riders fall into upcoming combat setup | Exit pocket, run forward, prepare to strike goblins | No long noninteractive victory scene. Dozer remains alive as stuck machinery, **not killable here** |
| **C8 — Immediate retaliation** | S5 first goblin encounter | One goblin's attack tell; first direct kill, later rock-shore opportunity | Punch, kick and use existing CP-03 stone if desired | Make **the first ordinary kill** satisfying in contrast with the Dozer's invulnerability |

These are *dramatic beats*, not fixed 8 scripted cutscenes. Each beat is triggered by real authored marker/collision facts. No scripted player displacement or camera-led death. One standout beat is sufficient: **the wheel passing inches in front of both heroes inside the alcove before demolishing the barricade**.

## 3. Preserve exact ENV-02 spatial interface

| Marker | Baseline | Required interpretation |
| --- | --- | --- |
| S1 / S2 / S3 / S4 / S5 / S6 | x 0–640 / 640–944 / 944–1712 / 1712–2608 / 2608–3296 / 3296–3584 | Do not mutate the authored segment order or add a second chase corridor |
| Chase start | x1072 | Start real pursuit once, after scene/retry proper capability setup |
| S3 lower ceiling | **option R**, ~212px clearance | Dozer visual height 155px; retain mechanical silhouette and no ceiling penetration |
| Charge arm | x1888, minimum gap ~140px in original spec | Dozer must communicate a **committed forward charge**, not turn/reacquire player mid-charge |
| Shared refuge | x2064–2160, raised 16px, **104px clear**, 80px usable | Fit both heroes, safe from Dozer's collision based on actual refuge geometry |
| Barricade | x2304–2400, full-height prop variant / gameplay blocker | Only the Dozer can clear it in the opening; no false player jump-over or early destructible exploit |
| Wedge | S5 arch x2608 onward | Need <=192px-wide **end-on/wedged pose** (or ENV-02 approved wall-breach fallback); do not squeeze a 259px side-on rig through a 192px window |
| Checkpoints | x1008 (chase), x2640 (post-chase) | Fast retry near danger; post-chase retry never reruns the entire chase |

Existing ENV-02 reserve geometry has been checked in authored tests; this spec requires the *actual Bevy, physics, camera and local co-op proof*, not an inference from key-sheet artwork.

## 4. Chase control and fail-safes

### Real motion, no fake cinematic threat

- Current ENV-02 hypotheses: cruise about **130px/s** vs Carl about **145px/s**; committed charge closes about **82px in 90 ticks** when initial separation allows it. DOZ-01 wheel/tread motion uses fixed tick and distance. These are **starting points**, not accepted encounter balance. Always derive actual elapsed time and gap in a deterministic simulation and tune against human inputs.
- Machine front wheel is the clear lethal contact zone; keep exact DOZ-01 safe-jump constraints. Do **not** make "jump over the Dozer" an alternative (approved apex sits below lethal wheel top).
- S3 timber is a one-way **presentation response to actual machine-front crossing**, not a new physics simulation. Splinter VFX never cause damage. Humps are still real simple traversal surfaces as authored; the machine visibly flattens them without player death outside its contact.
- Cruise cannot silently accelerate because the player catches up with the camera. Charge starts with a detectable tell (lever, whistle, wheel smear) and then commits; no hidden position snapping, instant adaptive pursuit or homing into a hiding player.

### Solo and 2P: the real risk

**The original "leading hero crosses x1888" trigger is potentially unfair in local 2P if the second hero is hundreds of pixels behind.** A fast player could arm the charge while P2 still cannot physically reach the x2064 pocket within the ~90-tick charge. This is a **design problem to solve**, not something to hide behind the camera leash.

Implementation should treat the x1888 marker as the **arm**; before starting the full lethal charge, prove every living hero has a reachable safe pocket/escape based on their actual location and measured movement. Compare candidate policies in deterministic simulations, then choose one with real co-op playtest evidence:

1. **Preferred experiment:** begin visible engine windup when the leader arms, but commit only when **both** live controllable heroes are in the physically reachable preparation band; if the rear hero is late, keep pursuit fair and readable instead of invisibly teleporting them.
2. **Fallback:** adjust the charge anticipation / measured gap / pursuer speed **within the same ENV-02 geometry**, preserving the wheel near-miss; the 8-second `barricade` fail-open safeguard remains a last resort to avoid softlock, but may never secretly force an unavoidable lethal charge on a straggler.
3. If no configuration fits extreme 2P separation, flag the geometry/camera conflict for owner review instead of silently changing core 2P camera rules, resizing the niche or giving one player fake invulnerability.

**Mandatory cases:** two real pads, both entering refuge; P2 0/96/224/448px behind at arm; reversed leader identity if party ownership supports it; one hero reaches niche first; solo Carl-active/Donut-active with the AI follower and correct recovery. The agent owning companion mobility is working separately: no new follower navigation/teleport mechanics in this task. An unavoidable 2P death at valid initial separation is a **release blocker**.

### Safety and difficulty

- A chase hit may lead to immediate failure or normal damage/retry according to existing player health rules; choose after gameplay feel tests. **Never** apply death because the renderer believes the Dozer is visually close; collision is authoritative.
- Keep the default chase doable from the first run without pixel-perfect jumps or secret interaction buttons. The x2064 niche must read as safety before the wheel commits.
- For first-time failure, allow a small **optional** contextual hint at the checkpoint ("GET INTO THE ALCOVE") only after one failure, and an accessibility setting if present. Do not make a big arrow or intrusive scripted prompt mandatory on a fresh run.
- Retries **restore all destructible visual props and barricade collision** and the Dozer's deterministic motion, but allow the player to resume at x1008 without waiting for the lengthy quiet intro.

## 5. Camera, audio and VFX direction (presentation-only)

### Camera

- Retain 640×360/640×400 native pixel presentation and existing integer snap / smoothing policy. The chase should show the machine **in frame well before contact**, without permanently zooming out or introducing cinematic black bars.
- In S3 prefer stable **forward travel framing** with enough rearward context to see the wheel as it closes; tolerate showing only part of the 259px body when necessary but never an offscreen lethal front.
- On x1888 reveal enough **forward** sightline to recognize the niche, then hold a composition that communicates (a) players inside, (b) wheel passing, (c) barricade ahead. Do not cut to a camera angle that removes agency. A minor presentation-only shake at barrier impact is acceptable if it never changes hit boxes or fixed-tick state.
- For 2P, actor separation and stop/overshoot camera clamps are harder than solo; record 640×360 and 640×400 captures at the peak charge and the barricade crash. Any camera-rule change outside existing ADR/contract needs separate approval.

### Audio

The repo contains `assets/audio/floor01/music/floor01_dozer_chase.ogg` and `assets/audio/floor01/sfx/murder_dozer_engine_sketch.ogg`. **Audit their current quality/approval and actual runtime availability**; treat them as starting material, not accepted final mixes.

- C0/C1: low mechanical vibration through otherwise sparse catacomb ambience; distinguish from ordinary combat via **negative space**.
- C2: a clear metal-wheel/engine entry stinger; chase music enters **on the gameplay trigger**, no arbitrary timeout from scene start.
- C3: two distinct spatially grounded timber breaks and an engine pitch rising; no permanent continuous wall of noise.
- C4: lever metallic click + whistle + rising tread cadence. It should be recognizable even at reduced VFX.
- C5/C6: rapid near-field wheel pass, loud **brief** single timber impact, then decreasing engine and wheel strain. Mix ducking should protect gameplay tells, avoid exhausting constant loudness.
- C7/C8: music **cuts or resolves** into relative quiet, then first goblin hit delivers a strong contrasting dry impact. Prefer an audible "we lived" moment over an extra cinematic scene.

All audio sync uses authoritative events; music/camera/particles never change simulation state, spawn, damage or fixed-step timing.

### VFX / art

- Only previously authored timber smash props, DOZ-01 wheel/tread/smoke and agreed opening markers are required in first pass.
- Splinters/shake/crumbs are **presentation**, capped and front/back layered correctly; show the wheel's bright spike ring against brick.
- End-on/wedged silhouette has an art gate; avoid stretching 259px side-on rig through the 192px arch, and avoid recoloring/replacing approved master sprites.
- **No slow-motion reward** for merely surviving the chase; cinematic slow-motion is reserved for exceptional skilled kills per CINE-01, and first victory occurs later in the goblin fight.
- Avoid inventing weapons, magic, audience/viewers or sponsor gifts in the opening.

## 6. State machine and event contract

Suggested **narrow chase-domain phases**: `Dormant` → `EntryTell` → `Cruise` → `ChargeArmed` → `ChargeTell` → `ChargeCommitted` → `BarricadeImpact` → `Overshoot` → `Wedged` → `Resolved`. Collapse phases if current engine has a simpler equivalent; avoid a giant general vehicle AI framework.

Authoritative events should include, where existing schema permits: `ChaseStarted`, `TimberSmashed(id)`, `ChargeArmed`, `ChargeCommitted`, `PocketEntered(hero)`, `BarricadeDestroyed`, `DozerWedged`, `ChaseResolved` and `CheckpointAdvanced`. These are semantic examples, **not** a requirement to break the Playtrace event schema. Distinct timber hits are one-shot by marker ID; barricade is removed **on the same sim tick** it is hit; the Dozer's wheel hazard turns off at the valid wedged/wall exit boundary. Preserve actual actor ownership and defeat/retry events.

Every gameplay state that can affect collisions, spawns or progression participates in the deterministic state hash and journal/snapshot contract; presentation-only VFX/audio do not. Repeat retries must reproduce exact authoritative transitions under the same inputs and ruleset. If the existing `floor01_scene` story route does not currently host combat/chase entities, wire it through the **current actual level/scene integration contract**, never build an incompatible duplicate scene framework.

## 7. Required real-gameplay QA

| ID | Scenario | Headless criteria | Human / native Bevy evidence |
| --- | --- | --- | --- |
| CHDIR-01 | Uninterrupted solo first run | Trigger→timber→charge→pocket→barrier→wedge→checkpoint deterministic | Understand danger & refuge without tutorial |
| CHDIR-02 | Carl tries to damage/clear Dozer | Does not decrement vehicle HP or allow wheel jump-through | Invulnerability clearly conveyed by sparks/impact and continued motion |
| CHDIR-03 | Two S3 humps, both timber sets | Timed movement and one break per authored ID | Two distinct physical destruction beats, not instant invisible path clear |
| CHDIR-04 | S4 committed charge | Telegraph precedes high-speed wheel; no homing into pocket | Charge direction and safe pocket understood before reaction deadline |
| CHDIR-05 | P1+P2 arrive together | Both fit niche safely; no accidental collision/shield errors | Both silhouettes legible, near miss exciting |
| CHDIR-06 | P2 behind by 96, 224 and 448px | Reachable refuge / no **unavoidable** charged-wheel collision in otherwise legal play | Real two-pad camera and recovery |
| CHDIR-07 | One player skips pocket / late retry | Fair failure+checkpoint, no grief/softlock; post-failure optional hint | Failure teaches rather than feels arbitrary |
| CHDIR-08 | Barrier hit and wedge | Same-tick blocker removal, single impact, post-x2640 safe | Big smash, then abrupt relief and direct first fight |
| CHDIR-09 | Checkpoint/reset at x1008 and x2640 | Initial timber/barricade/machine state after chase retry; no replay after post-chase retry | Fast retry; skipped long intro |
| CHDIR-10 | Both virtual resolutions and output scaling | No threat or warning cropped; no invalid zoom or actor silhouette distortion | At-speed 640×360, 640×400, 1080p/Deck-friendly |
| CHDIR-11 | Playtrace capture + verification | Whole live attempt/retries captured; same event sequence and state hashes on replay | Tick-aligned render evidence if supported |
| CHDIR-12 | Compare audio on/off and VFX reduced | Same authoritative state/hash and victory outcome | Threat cues remain accessible without full effects |

**Playtest questions:** Did they recognize the machine before it appeared? Did they know what to do? Was the hide→near miss→wood smash a real highlight? Was the chase frightening without feeling cheap? Did the first goblin kill feel like emotional release? Compare **three runs per mode** (first clear, retry, practiced) before freezing timings. No auto-player or ML success score constitutes a human "fun" signoff.

## 8. Implementation phases / handoff

1. **CH-A — Honest base:** audit actual Dozer art integration, `opening_route_proposal.json`, real scene entry, current party/camera and checkpoints. Expose a playable S1–S5 route (test entry if story wiring absent) without new parallel scene engine.
2. **CH-B — Fixed-step chase:** spawn/turn-in, wheel hitbox, cruise + S3 break events, 2 humps and retry. Unmodified current controls.
3. **CH-C — Spectacle core:** S4 charge/tell, pocket collision and two-player fairness, barricade impact, overshoot/wedge and immediate goblin encounter. The full passage must be human-playable before audio polish.
4. **CH-D — Sensory direction:** existing approved native sprite rig and prop variants, restrained audio/VFX/camera punctuation, accessibility and graphics variants.
5. **CH-E — Evidence and review:** deterministic fixtures, live Playtrace verification, real two-pad footage, measured durations, change log of tuning hypotheses, independent review + owner approval.

**Out of scope:** Dozer revenge, new enemies, bomb/gauntlet/Donut ability work, companion navigation implementation, new camera architecture, new cinematic fatality system, special loot/HUD, general mutable destruction engine and full Floor 01 route completion.

**Local agent:** [dedicated work order](../07-agents/floor-01-dozer-opening-workorder.md). No feature is considered implemented merely because this design document is merged.
