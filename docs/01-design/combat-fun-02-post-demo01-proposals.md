# COMBAT-FUN-02 — make Floor 01 combat worth replaying

> **Status: DRAFT design proposals, not approved gameplay tuning or implemented features.**
> Owner's first hands-on DEMO-01 review: 2026-11-20. The owner requested a concrete design PR informed by the game's existing references and principles. This document proposes *experiments* and recommended starting choices, not a new mandatory controller contract. The [recorded playtest](../04-production/demo-01-playtest-2026-11-20.md) is the evidence; the [agent work order](../07-agents/combat-fun-02-workorder.md) sequences delivery.

## 1. Problem and target experience

The current demo proves that a deterministic, lit Bevy encounter can be completed; it does **not** yet prove that playing it is consistently fun. The owner's recurring complaints are: basic repetitive fighting, enemy-to-enemy impacts with little perceptible consequence, limited evasive options, Donut contributing too little, weak/undifferentiated visible deaths, and flat encounters. Tone/identity rated 2/5. This is a **game-feel failure**, not evidence that the code for contact, gore or finishers is absent.

**North-star test:** repeat the *same 40–60 second fight* three times voluntarily because the player can invent a different satisfying sequence each time. The fight should be exciting with only Carl and goblins, then become more interesting with Donut and with the terrain; killing an enemy by hitting it repeatedly must remain viable.

### Authority and influences

- [Vision](vision.md) and [combat-first encounter direction](combat-first-encounter-design.md): basic takedowns must feel good; physical and environmental causality; player-authored mayhem; short recovery, not sponge enemies or puzzle-first rooms.
- [Combat feel / hybrid physics](combat-feel-hybrid-physics.md): fixed-step authoritative outcomes; authored, expressive cosmetic reactions; bounded contacts and one-hit identity; no uncontrolled ragdolls.
- [MVP controller contract](mvp-controller-contract.md): South jump, West primary, North physical, East story-unlocked special, R1 interact, L1 solo swap. No mandatory dodge/grab/sprint/twin-stick buttons. Carl never grabs enemies.
- [Antihero tone](antihero-tone.md): a scrappy, imperfect everyman winning by effort, improvisation and accidents, not flawless superhero martial arts. Original 1980s–1990s action atmosphere and vicious situational comedy rather than copyrighted scene/music/dialogue imitation.
- [Character progression horizon](character-progression-horizon.md): early Carl punches/kicks and environment; pre-biscuit Donut uses cat-like swat/pounce/mobility; Magic Missile only after the biscuit; earned explosives only when fiction allows; no early-floor Mongo or unapproved late powers.
- Our reference vocabulary: **Broforce** for urgency, readable destruction and emergent surprises; **Dead Cells** for quick response and distinctive hit feedback; **Mewgenics** for character expressiveness and reusable authored animation; **Noita** for meaningful material interaction **without** reproducing its per-pixel simulation. These are *principles to evaluate*, not an instruction to copy assets, exact moves or rules.

### Non-negotiable boundaries

1. No extra required face-button verb and no redesign of the approved controller layout; context/direction/state may vary an existing move only when communicated clearly. Remapping/accessibility retained.
2. No early Magic Missile, dynamite, Puddle Jumper, shoulder mode or other story-locked skills; demo **probe flags may explicitly grant post-unlock capabilities** for tests only and must not leak into the story route.
3. New combo depth = **branching consequences**, not memorized ten-hit strings or higher enemy HP. No arbitrary melee decapitations, blast kills, or slow-motion rewards from ordinary hits.
4. Preserve the visual 1x/Deck/1080p readability and approved Carl/Donut sprite identities, silhouettes, foot pivots and established industrial catacomb art. Never use concept PNGs as production sprites.
5. Same combat opportunity and solvable encounter in solo and local 2P. The solo AI must not steal the player's wins; P2 must not wait as a spectator.
6. Actor/terrain collision, damage, attribution, cooldowns and kill causes stay deterministic at 60 Hz. VFX, screen shake, camera zoom, audio, gore preset and cinematic slow motion never write back to gameplay or its replay hash.
7. The visible live-viewer HUD (AUDIENCE-01 #118) **remains absent at the start of Floor 01** and is outside this iteration. Do not use it to conceal a weak base combat loop.

## 2. What we can reuse, and where the evidence stops

| Existing system | Verified repository status at the time of authoring | Gap identified by playtest |
| --- | --- | --- |
| Carl jab -> haymaker, push kick, hit-stop, goblin tell and stagger | Live in DEMO-01; CP-01 / ANIM-CARL-02 | Owner 3/5, says few attacks and monotonous kills |
| CP-02 living goblin body contact | Deterministic contact/knockdown in DEATH-A (#79), demo B2 | Owner **1/5** for impact-on-enemy; distinguish a missed trigger from a visually weak consequence |
| CP-03 shore, falling block, B3 decap and B5 QA charge | DEMO-01 + DEATH-D (#108) | Environmental kill 3/5; player did not see decap or blast; do **not** call this proof that they cannot fire |
| Death cause/profile/presenter and preset 4 | DEATH-B/C/D/E1 (#94/#97/#108/#106) | Gore intensity **1/5 too mild**, clarity 3/5, aftermath quantity 3/5: enhance silhouette and cause, **not** the number of permanent floor stains |
| CINE-01 exceptional slow motion | Merged #122; skill-gated; normal kills and boss deaths excluded | No valid owner verdict yet; keep rare and separate from gore tuning |
| Solo swap, 2P, follow-and-jump bot | Merged #114 | Swap 2/5, AI 2/5, real two-pad play **not tested** |
| Donut swat/pounce, biscuit mana and Magic Missile | **PR #123 open**, not in main at this baseline | Controlled Donut gains attacks after integration; AI follower **still starts no attacks** by deliberate Q9 rule; avoid duplicating #123 |
| HUD, camera and SFX | #103, #112, #80 | HUD 3/5; zoom not observed, sometimes abrupt motion; some impact, body-contact and heavy-prop cues not hooked up |

Do not assert a player's exact build, device or graphics settings: none was recorded. Before making claims about defects, collect the play-session commit/output/flags or reproduce on a known commit.

## 3. Proposed mechanics and two-option experiments

### CF02-A — Short, branching Carl combos and meaningful downed targets [P0]

Keep the quick jab, chained haymaker and heavy kick. Add **two tactical continuation opportunities**, not a long combo tree:

- **Hit-confirm branch:** a connected quick strike opens a short, forgiving window for either another primary (follow-through) or Physical (strong directional kick). A whiff does not grant the same advantage. First tune with existing animations/hitboxes; author new native sprites only after the mechanical comparison proves worthwhile.
- **Ground branch:** a *living, downed* ordinary foe is vulnerable to a distinct short-range Carl stomp/finishing kick, with its own anticipation and recovery. A routine standing jab should **not** magically stand the downed foe upright. Existing ordinary kick remains available for relaunching; choose one of two test mappings: A) **Down + Physical** requests the stomp and Physical alone remains kick; B) Physical contextualizes to stomp only inside a short, clear ground-target zone. Compare accidental activation, accessibility and feel; do not pick a binding without evidence.
- The alternative deliberate sequence is **jab -> haymaker -> kick toward a prop**, while **kick -> down -> stomp** cashes in a positional advantage. Donut's swat/pounce can interrupt and complete a different sequence after #123.

Guardrails: per-instance hit dedup, bounded stun, no infinite ground loops, no guaranteed instant executions, and no stun-lock of bosses. Ordinary goblins remain quick to defeat. Keep Carl's physical effort and imperfect motion; no stylish superhero spin-kicks by default.

**Probe:** three goblins in a compact room; compare legacy attacks to branch A/B. Measure input drops, accidental stomps, distinct viable player sequences, completion and human enjoyment. Record a 1x/speed GIF and a replay trace showing each branch.

### CF02-B — Impacts that create an *observable* second decision [P0]

CP-02 already moves a second goblin. Keep the existing one-shot contact identity and no auto damage on every overlap. For a **living** launched enemy with sufficient bounded momentum, propose:

- Contact produces a **clear recoil/stumble pose**, chunky contact sound, brief separation and a readable recovery window for the second goblin. Make causality unambiguous even when no damage is applied.
- A wall collision is visually different from enemy collision; a brittle fixture may react when its material and authored rule allow it. Provide a *single*, intentionally designed follow-up opportunity to hit or kick the staggered target.
- Compare A) one-shot stagger only (the safest baseline) with B) single knockdown at a *higher* impulse threshold. No damage from detached body pieces; cosmetic gibs never become invisible damaging projectiles.

**Probe:** B2 with paired goblins and an obstacle. Log ordered launches/contact ids, resolve exactly once, verify no cycles/duplicate impacts, and capture a first-person-visible result at 640x360. Target the owner's 1/5 rating, not an arbitrary impulse increase.

### CF02-C — Donut agency and solo-swap responsiveness [P0, after #123]

**Do not reimplement swat/pounce/mana or silently change the current Q9 companion decision.** First merge/review #123 by its normal independent-review/CI process. Then run **two controlled AI variants** over the same fixed input:

- A **current safe follower**: follows, avoids tells, never initiates attacks.
- B **restrained combat assistant**: may *swat to interrupt* an immediate threat to the controlled hero or *pounce a specifically exposed, nearby target* under strict cooldown, range, visibility and no-overkill/last-hit conditions. It must not spend story-locked mana, act off-screen, attack unprovoked or erase the player's setup. On direct control the player retains all actions; on changing to AI finish an in-flight move but never spawn duplicate hits.
- Solo swap comparison: current 20-tick cooldown + 6-tick buffer + 8px/tick pan, against a shorter/faster but safe alternative. Avoid switch thrashing, hurt exploits, input loss and camera snapping. Do not alter 2P input ownership or change the party-defeat rules.

**Probe:** identical threat wave as solo Carl, solo Donut and 2P; quantify AI interrupts, kills credited to player versus AI, misses, player downings, party separation, swap lost inputs and camera displacement. Human test must answer whether Donut feels helpful **without automating the combat**. In 2P Donut must have an active combat decision even before biscuit.

### CF02-D — Defeat language that really communicates force [P0–P1]

The owner's 1/5 for preset-4 intensity and not noticing either special finish calls for **cause-based expressiveness**, not blanket gore saturation.

1. **Audit observation first**: on a known build, display/capture B3 shear, B5 charge, kick impact and llama heavy-vs-routine finishing at the actual 640x360 view. Record event logs versus camera visibility. Classify each as trigger not reached, trigger reached but off-screen, hidden by world/HUD/VFX layering, or visible but unreadable.
2. Improve **big silhouette events**: a goblin crumples on a heavy wall hit, a kicked head/limbs move distinctly where an existing species profile permits, impact poses hold long enough, debris has coherent trajectories, a blast breaks the body with meaningful visible negative space. Contrast *ordinary defeat*, *heavy environmental kill*, *precision decap*, and *explosion*. Preserve anatomically appropriate fallbacks for slimes/animals/armor.
3. Wire cause-specific original SFX where missing (enemy contact, block impact, explosion and defeat). Evaluate the reused jab/haymaker 'kick' cues and provide distinguishable original transients before raising global volume.
4. Keep intensity preset 4 as the owner's chosen default. It controls **cosmetic intensity**, not force tier, actual damage, rarity, eligibility or the authoritative event set; retain presets 1–3 for accessible presentation. Do not increase retained bodies/decals merely to raise spectacle.
5. CINE-01 stays a separate **rare, skill-gated reward** for truly authored exceptional kills. Do not trigger slow motion on every gore explosion or routine kick; test co-op slowdown comfort and reduced world-motion compatibility.

**Probe:** contact sheets/GIFs with original pixels at 1x and 3x; at least three clearly differentiated causal outcomes identified by a viewer without looking at logs. Full replay/event/hash parity between gore presets and cinematic on/off.

### CF02-E — Fair defensive choices against the llama [P1]

The owner rated the llama tell 3/5 and felt unable to evade in enough ways. Do **not** add a mandatory dodge button by assumption. Compare three alternatives:

- A **current jump + positioning**, with better tell, projectile speed/height/collision and a brief readable counterattack opening. This is the minimum baseline.
- B **movement/air-control tuning** and purposeful recovery cancel for an existing verb, enabling ducking under or vaulting over a telegraphed attack via terrain rather than input bloat.
- C an **optional directional/context evade** only if A/B fail, with an explicit, collision-safe command and a decision on invulnerability, cooldown and conflict with the four approved face verbs. C needs a separate input-contract design review, not silent implementation.

**Probe:** llama on level ground and on a raised platform; approach from both sides and from an overhead perch. Two viable player-authored responses must be perceivable and executable with a normal controller; dodge must not negate danger or create an unavoidable attack in 2P.

### CF02-F — A *real* combat microarena with authored choices [P1]

The DEMO-01 hall is a systems proving ground. Keep it for regressions and author a **new 40–60s microarena** from existing ENV-01/02 source assets rather than reusing one long flat corridor.

Suggested topology: readable central low brawl lane; one **reachable upper perch** for Donut; a **breakable timber support plus telegraphed falling load**; a **hard, nonbreakable wall** for knockback and a small safe pocket/cover against the llama. Display foreground/back layers clearly and leave enough full-height art above the widest camera view; no void at 960px world width or 640x400 Deck.

At least three meaningful solutions to the *same* scenario:
- **Direct:** jab/chain/kick and move through the brawl.
- **Environmental:** lure/launch a foe to the load and break the support.
- **Cooperation:** Donut interrupts/positions, Carl follows up; in solo a swap or opt-in assist creates an equivalent opportunity.

**Owner-approved new test:** the falling load can hurt Carl (and any other actor based on the adopted friendly-hazard rule), but only with an anticipatory cue, spatially clear footprint, avoidable timing, safe spawn and checkpoint; no invisible player instant-death, accidental softlock or off-screen unavoidable drop. Exact damage, timing and hit policy are **not frozen** here.

Add one brief authored absurd beat (e.g., machinery betrays a goblin after an ill-advised attack) and a restrained reaction that makes the room feel like a cruel staged spectacle. No invented book dialogue, no premature viewer counter and no need to add a new monster.

**Probe:** 1P with swap, 1P with follower, real 2P, 1080p and 640x400, with collisions/reachability validated on every supported destruction state.

## 4. Minimal release sequence and conflict ownership

| Stage | Owner / isolation | Depends on | Deliverable |
| --- | --- | --- | --- |
| 0. Baseline and compatibility | Integrator | main + PR #123 actual status | pin playtest evidence; reconcile current branch, preserve DEMO-01 golden; no claims about owner's unrecorded build |
| 1. Combat depth | Gameplay domain agent | baseline | CF02-A short combo / grounded-finisher **comparison**, deterministic tests |
| 2. Contacts | Second gameplay agent on separate worktree | approved contact event schema | CF02-B visible second consequence, event/replay tests; integrator serializes combat schema merges |
| 3. Party/AI | Party agent | #123 merged and synced | CF02-C opt-in AI A/B and solo-swap variants, 1P/2P replay and captures |
| 4. Death/presentation | Art/VFX + audio agents, separate source folders | authoritative event shape frozen | CF02-D original editable effects and cue mapping, independent art review; no new gore gameplay flags |
| 5. Defense, microarena | Level/design + integrator | A/B/C basic behaviour available | CF02-E A/B tests; CF02-F one authored arena; *no* full floor expansion |
| 6. Integrated review | Integrator + owner | all passing gates | COMBAT-FUN-02 playable scenario + matched replays, visual and audio evidence; owner solo & 2P re-evaluation |

**Avoid parallel writes** to `src/combat_scene.rs`, `src/bin/floor01_scene.rs`, the canonical goblin sprite master, input schema or shared tuning tables. Keep pure Rust modules independent and land Bevy adapters sequentially with named integration boundaries. The agent workorder specifies evidence and handoffs.

## 5. Acceptance, measurement and stop criteria

**Human playtest targets (proposed, not automatic pass/fail gates):**

| Item | DEMO-01 | COMBAT-FUN-02 target |
| --- | --- | --- |
| Basic combat fun | 3/5 | >= 4/5 and owner voluntarily repeats the encounter to vary the outcome |
| Enemy-to-enemy impacts | 1/5 | >= 4/5; visible first and second body responses |
| Gore intensity | 1/5 (too soft) | 3/5 (right intensity on the original scale) |
| Death readability | 3/5 | >= 4/5; 3 causal classes named accurately without logs |
| Solo swap / companion | 2/5 / 2/5 | >= 4/5 / >= 4/5 |
| Identity | 2/5 | >= 4/5; remembered authored situation rather than generic combat |
| HUD / camera | 3/5 / zoom unseen, abrupt motion reported | HUD >=4/5; measured pan/zoom trace plus no disorienting snap |
| Checkpoint / real 2P | not tested | must be tested; **no invented numeric baseline** |

**Engineering gates:** original 60Hz headless scenario still replays; unchanged fixtures preserve hashes, or any *deliberate* migration is documented old -> new with event rationale and new goldens; no presentation flag alters authoritative events; bound contact/debris pools; no softlocks when supports are broken; same room solvable in 1P/2P; observed controller input ownership; original editable sprites when needed; 1x/640x400 render evidence, clean format/clippy/unit/docs CI and per-PR independent review. Frame-time baselines on target hardware rather than assumed performance.

**Stop / simplify if:** attack variants mostly whiff due to input buffering, a stomp steals ordinary kicks, the companion dominates kills, chain contacts cause repeated damage or impossible recovery, camera framing hides threats, color/particles conceal the attacker, or the arena requires an unapproved ability. Narrow the experiment before adding content.

## 6. Decision ledger (what is decided and what is not)

**Owner observed or requested:** more combos and attacks; ground advantage via kicks/stomps and Donut claws; environment may injure Carl; more physically broken bodies without simply piling up stains; meaningful evasion; richer authored spaces; Donut needs to contribute. These are confirmed priorities, **not an approval of the particular implementation variants here**.

**Recommended starting hypotheses:** A and B first for evasions; stagger-only first for contacts; keep existing kick as the default when ground context is ambiguous; opt-in AI assist variant B for test only; use existing death recipes and species parts before commissioning a new system. Exact damage/ticks/ranges/bindings, consent to automatic companion attacks, any new input contract, optional camera policy and authored hazards' friendly-fire rules remain subject to separate review.

**Horizon check:** all proposed tests fit the current early Floor-01 progression and no unearned powers are granted; if a later design expands Donut's guaranteed auto-assist or Carl's special inputs, revisit the horizon and controller contract *explicitly*.

**Out of scope:** audience counters (#118), end-stage results, subscriber economy, full boss tuning, new enemy-roster production, Dozer/Guild full narrative route, full destructible pixels, procedural arenas, new monetization/loot, and commercial release rights.
