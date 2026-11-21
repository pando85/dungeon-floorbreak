# F01-SETPIECE-DOZER-REVENGE — "The Machine Remembers"

**Status:** original adaptation pitch approved as a memorable-scene direction, but **actual encounter/story beats, mechanics, exact timings and vehicle damage states are hypotheses**. Design-only.  
**Location:** late Goblin Workshop, after BOILER-01, **only once Carl's narrative dynamite has been earned**. Duration target **60–100 seconds**, optional vehicle defeat; do **not** promote this encounter into a second conventional boss health-bar fight.  
**Memory payoff:** the opening Murder Dozer was an overwhelming escape hazard. A later machine of the same recognizable family is terrifying for the first few seconds—until the player recognizes that the environment and earned abilities now offer real counterplay.

## 1. Visual / story constraints

- Preserve the **approved h120 Murder Dozer rig and its distinctive tracked wheel** as visual reference, not a duplicate/new huge vehicle. The opening DOZ-01 explicitly has **no death/damage sprites or Bevy wiring** in its original art status; do not claim those exist. Any new damage/disabled animation or gameplay integration requires its own native asset review.
- Avoid unverified assertion that the returning machine is the *identical* vehicle or the same riders from the opening. Label it **another Dozer of the same family** until the narrative team/source review makes a stronger claim.
- Reuse one recognizably identical **approach sound / wheel tell** from the opening for emotional recognition, without depending on sound for safety.
- This is a **new adaptation set-piece**, not a claim that the novel has an exact revenge scene; a non-canonical moment may need narrative approval before entering the definitive story route.
- Maintain the workshop/catacomb visual grammar; large press frame and scrap barriers are concentrated in this room, not a novel biome.

## 2. Room shape: an arcade escape with an optional counterstrike

Suggested authored horizontal width **1280–1440 world px**, fixed protected floor lane, side-view scrolling camera consistent with Floor 01 camera/party contracts. This is a layout hypothesis; fit the actual Dozer rig width, controller jump arcs, shared-screen framing and scene data before locking coordinates.

| Area | Functional geography | Player affordance |
| --- | --- | --- |
| R0 — Familiar rumble | Safe 128–192px entry; rear gate/exhaust begins to tremble | The first 1–2 seconds build anticipation without a surprise hit |
| R1 — Chase lane | Long clear floor ~320–480px, with only low jumpable steps/escape cover | Run from left-approaching lethal spike wheel; no false "jump over Dozer" affordance |
| R2 — Elevated bypass | Short overhead maintenance walk with ground-access ramps/steps *before* the wheel catches up | **Guaranteed safe forward exit** even if the player ignores all machinery |
| R3 — Counterstrike apparatus | Obvious **sacrificial industrial stop/press brace** close to the vehicle's path and reachable to a player without standing under the spikes | Breakable timing interaction; the approaching machine meets a real collider |
| R4 — Exit/regroup | Protected open route right of the stop; leave space for both 1P and 2P | Campaign continues if vehicle escapes; cannot softlock through collapse |

**The player is never obligated to disable the Dozer.** A skilled player can exploit the apparatus to disable it and confront the occupants; a new player can escape, preserving the opening threat identity. Normal dodge, run and jump suffice for escape. There must be no mandatory ranged or precise timed dynamite throw that only one hero can perform under impossible pressure.

## 3. Beat structure

| Beat | Duration target | Gameplay and emotion | Nonnegotiable |
| --- | --- | --- | --- |
| R0 — Recognition | ~1–2 seconds | Audible/visible same-family wheel tell; Dozer enters behind | Do not attack from beyond camera with no forward escape |
| R1 — Flight | ~10–20 seconds | Chase drives forward and recalls opening fear | Chase speed/viewport preview gives a tested, fair margin; protected bypass |
| R2 — Realization | ~8–15 seconds | Visible dangling stop frame/brace; optional dynamite/physical action can release it | Not a button prompt, not a forced stage puzzle |
| R3A — Escape | Player choice | Continue through safe route, Dozer remains behind / is restrained off-path | Fully valid completion |
| R3B — Counterstrike | Player choice | Stop frame **physically contacts** front wheel/body, halts Dozer, shakes crew loose | Real contact/telegraph, no magnetic catch or prescribed auto-win |
| R4 — Payoff | ~10–25 seconds if counterstrike | Surviving goblins tumble into a small, ordinary combat pocket and can be fought | No free scripted enemy deaths; a non-disabled machine never sheds free killable riders |

The counterstrike should visibly show the front roller jamming, tread overrun, hull pitching and engine smoke—**animation responses to actual domain states**, not an extra lethal cinematic cutscene. Optional "brief victory" audio cue should be shorter than the action itself.

## 4. Minimal gameplay state machines

### DOZER-R (later machine; do not modify opening chase semantics)

`approaching` → `committed_chase` → (`escaped` OR `impact_stunned` → `disabled`). Phase transitions are driven by camera-safe position/actual collisions. `impact_stunned` does not become an ordinary HP pool; the *vehicle is a positional hazard*, not a punchable goblin.

- Wheel remains a large lethal, telegraphed contact hazard. Original DOZ-01 h120 art warns that Carl cannot safely jump over the front spike circle; do not script a false jump-over route in late-floor geometry.
- Proposed initial chase speed **150–195px/s** (not an approved final value). Derive fair margin from real run speed, earliest safe turn, two-player separation, and initial camera composition. The vehicle must not outrun the players by default or become trivial if slightly behind.
- If the press frame misses the vehicle or is never released, escape remains viable; **no endless reappearing Dozer behind the player**.
- Riders and crew become normal combat actors **only after** a genuine disable/crash event; first pass can use already integrated common goblin behaviors, postponing engineering-driver-specific attacks to a separate approved feature.

### Industrial stop frame

`raised` → `brace_weak` → `released_warning` → `descending` → `landed`. It is a **single-use** encounter fixture, not a general physics contraption.

- A ground-level, visually obvious vulnerable brace can be hit using an already available Carl action; dynamite can also release it if earned. Because Donut cannot normally destroy the brace, the **mandatory route is always the bypass**.
- Proposed warning **24–36 fixed ticks** with audible creak, clear floor shadow/impact footprint and bounded falling mass.
- Disable Dozer **only on real swept intersection** between stop frame and machine's declared impact collider; falling too early/late is a valid miss, and weight cannot instantly chase the Dozer position.
- Landing near a hero can deal honest warned damage; no invisible dynamic trap or infinite damage after the mass rests.
- Protected exit and raised bypass persist even after destruction; future replay/retry spawns the frame raised, the same machine and same seed.

### Riders / post-impact combat

On disable, spawn or dismount a **bounded** number of existing goblin actors with source attribution and fixed locations derived from the crashed vehicle position; never teleport new enemies far into the player or produce more than the room's validated concurrent cap. The encounter may end by exiting without clearing them if design tests show that pressure improves the pace. If the dismounted enemies are a reward fight, display them in reach and give the player normal interrupt/kick/impact responses.

## 5. Why this encounter is distinct from the Goblin Boiler

The boiler tests **tactical chain reaction in one wide arena**. Revenge tests **moving threat anticipation** plus an optional skillful reversal under time pressure. It should not repeat the valve/steam/weight/drum four-prop loop. Reuse **shared primitives** (authored support, swept falling collider, damage attribution, debris, camera tells), but add only the Dozer's positional chase/disable state and small rider finish.

## 6. Win paths and fairness

| Approach | Valid outcome |
| --- | --- |
| Run and escape via clear bypass | Full story progression, no mandatory victory cinematic |
| Timed brace release | Vehicle disabled by real physical contact; optional crew fight |
| Dynamite to fixture | Different input to the same honest physical interaction; not an instant global HP reduction |
| Mis-timed release | Environmental object visibly falls, misses, and player still escapes |

No party mode may depend on two humans timing an input simultaneously. Shared-screen 2P must never force the rear player into instant wheel death while the front player can escape. A solo AI companion must not be relied upon to hit a timed brace. Use regroup/recovery rules from existing systems; do not modify companion movement in this work.

Do **not** require the post-biscuit automatic dodge to nullify the lethal wheel; its classification should be explicit, readable and reviewed separately from that independently owned mechanic. No special invincibility for Donut, no player-killing offscreen collider. The game should not imply repeated dodges of a 120px spiked vehicle are the intended answer.

## 7. Repeatable tests / evidence

| ID | Test | Expectation |
| --- | --- | --- |
| DOZR-01 | No counterstrike, only run/jump | Every control mode can reach protected exit without touching apparatus |
| DOZR-02 | Brake frame released well before approach | Visible miss; Dozer motion/escape still coherent |
| DOZR-03 | Brace released at authored timing | Swept real contact transitions machine to disabled exactly once |
| DOZR-04 | Dynamite to brace | Only already earned special releases; attribution preserved |
| DOZR-05 | Strike vehicle armor/wheel with standard attacks | No false damage state or unrealistic jump over wheel |
| DOZR-06 | Disabled rider tumble | No free death, no offscreen spawn, exactly bounded living fighters |
| DOZR-07 | Restart at each chase/impact phase | Machine, frame, crew and terrain reset identically; trace hash matches |
| DOZR-08 | Solo Carl, solo Donut, local 2P | Same authored path, fair camera/spacing, no AI-dependent timed activation |
| DOZR-09 | Performance & real-pad playtest | Opening encounter recognition, optional reversal and fast retry |

Capture **escape**, **miss**, and **successful reversal** playtraces with real game footage or native Bevy frame evidence. Automated testing proves safety and determinism, not a human "wow" verdict.

## 8. Dependencies / recommended implementation stages

- **Preflight:** audit `docs/02-art/doz-01-dozer-spec.md`, the opening Dozer behavior, its actual native asset/export state, current `floor01_scene` entry/camera, active companion/capability PRs and the story chronology. The original Dozer's initial-purpose "escape threat, not boss" must remain intact.
- **DR-A:** build a small runnable escape room with placeholder machine collider matching approved h120 art; no new props, prove 1P/2P path and camera.
- **DR-B:** one data-authored stop-frame/brace with contact and reset, preferably sharing a tested domain primitive with BOILER-01 after it merges. If not ready, keep behind explicit feature gate; no parallel duplicate falling-block engine.
- **DR-C:** authored disable/vulnerable crew aftermath; any fresh art or animation is separately reviewed.
- **DR-D:** a short time-pressure replayable version, camera/FX/audio and human playtest. Target under ~100s, no traditional boss HP bar.

**Scope:** design work only in this PR. Concrete level/logic PR must be independent of BOILER-01 and should start only after checking interface stability. Sources: `docs/02-art/doz-01-dozer-spec.md`, `docs/01-design/floor-01-act-01-playable-combat-beats.md`, `docs/01-design/combat-first-encounter-design.md`, `docs/01-design/floor-01-goblin-boiler-encounter.md` (on PR #176), `docs/01-design/asymmetric-traversal-and-gates.md`, `docs/03-tech/playtrace-01-session-capture.md`, existing art and rights rules.
