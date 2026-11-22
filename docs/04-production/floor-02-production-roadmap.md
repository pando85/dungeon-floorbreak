# Floor 02 — Production roadmap, gates and evidence contract

**Status:** proposal 2026-11-22; game-feel owner playtest of Floor 01 is planned **before travel**. This document is not evidence of delivered Floor 02 gameplay. See [design](../01-design/floor-02-last-lesson-design-bible.md), [visual direction](../02-art/floor-02-concrete-hell-visual-direction.md), [autonomous local-agent work order](../07-agents/floor-02-autonomous-workorder.md).


## Production unit and chapter boundaries

**Player-facing release structure = five connected arcade chapters (A–E), made from ten internally named S00–S09 encounter beats.** These are not ten mandatory stage-clear panels. A chapter's checkpoint and story-grant boundary must be explicit; a beat may have an internal retry checkpoint for failed boss attempts.

- **Chapter A:** S00/S01 (arrival and first Mind Horror).
- **Chapter B:** S02/S03 (Rev-Up and Krakaren; physical setpiece).
- **Chapter C:** S04/S05 (corpse ecology and Meadow Lark convoy).
- **Chapter D:** S06/S07 (Rage refuge and stairwell run).
- **Chapter E:** S08/S09 (kennel, Ralph, Mongo and exit).

The targeted high-risk implementation specs are [F02-RK-01](../01-design/floor-02-revup-krakaren-setpieces.md), [F02-MR-01](../01-design/floor-02-meadow-rage-setpieces.md), and [F02-RALPH-01](../01-design/floor-02-ralph-kennel-finale.md). **Preserve a seamless linear experience**, with no forced pause while danger is active. Pacing and individual chapter count can be revised after real playtesting.

## Design briefs added to the delivery contract

- [Arrival / Psychic / Cleanup greybox](../01-design/floor-02-arrival-cleanup-encounters.md)
- [Enemy acting and animation bibles](../01-design/floor-02-enemy-acting-bible.md)
- [Chapter director, score, checkpoint and stairwell continuity](../01-design/floor-02-director-and-continuity.md)

**New P0 story-graph invariant:** S07 Rage exile remains on Floor 02; only S09 grants player Floor 03 descent (DIR-03). **New adaptation:** generic Rev-Up foreman role is visually cast as the lore-sourced Laminak executive; do not implement redundant bossy enemy types. Visual and audio direction do not imply an approved shipping asset.

## Difficulty composition and QA additions

- [F02-COMPOSE-01 — tactical pairings and authored pressure curve](../01-design/floor-02-encounter-composition-and-difficulty.md) is authoritative for scene roster choices, not a green light for automatic difficulty tuning.
- [F02-QA-01 — native AUTOPLAY and PLAYTRACE evidence plan](../03-tech/floor-02-automated-encounter-evaluation.md) defines direct/prop/failure comparisons, meaningful baselines, false-confidence/negative tests and human-vs-automation boundaries.

**P0 release checks:** no unreachable direct path, no unavoidable overlapping attack with no legal counter, no ability leak, no weak/damaged env duplication, no camera/party softlock, no S07 floor advance. **P1 investigations:** stationary Donut trivializes mixed waves; companion passive; environmental chain causes no genuine distinct gameplay result; Ralph too small to hit fairly. **Human gate:** game-feel and real two-pad play remain unmeasured until owner session. Preserve current Floor 01 caps and baseline before making cross-cutting changes.

## Week-away implementation/owner entry packet

- [F02-ARCH-01 — verified current Rust and JSON seam audit and floor-neutral route approach](../03-tech/floor-02-engine-reuse-and-route-contract.md).
- [F02-EXEC-01 — W00–W16 execution DAG, PR sequencing, parallel lane ownership, fallback work and return packet](../07-agents/floor-02-vacation-execution-packet.md).
- [GAMEPLAY-BASELINE-V1 — unfilled pretrip owner test template](floor-02-pretrip-gameplay-baseline-template.md), **not proof of any owner pass**.

**Decision:** prioritize a real, connected S00–S03 vertical slice (intro + Mind Horror + Rev-Up + Krakaren) and its direct/environment counterfactuals before mass-producing content. If Floor01 E2E is not implemented, this may be an F02 story-safe isolated entry, **not** a falsely claimed full campaign transition. Pure logic, original art and authored tests may progress during a known shared-core blocker. Every scene carries separate source, bot, renderer and owner statuses.

## Product objective

A **linear arcade-combat** Floor 02 that feels visually independent from Floor 01, stays replayable, uses physical collisions and authored destructible props, has two direct-combat bosses (Krakaren Clone and Ralph), one **unbeatable rule-based flight** (Rage Elemental), a protective Meadow Lark setpiece and a quiet Mongo introduction. All floor-specific work should be usable as isolated probes before connecting the complete route.

**Do not measure progress as code lines or merged PR count alone.** Metrics: # independently playable units, complete story-reachable route, replay stability, real-hardware frame-time, direct clear route, game-feel verdict, and number of undisclosed dependencies/hot-file conflicts.

## Entry gate: GAMEPLAY-BASELINE-V1, before travel

Human playtest of the newly merged Floor 01 slice, preferably with 2 real controllers:
- Carl melee and air kick response, actual body-to-body damage, dynamite and reinforced fist.
- Donut manually controlled and autonomous: actual useful attacks, evasive behavior, camera, regroup, cooldown feedback.
- 1P character swapping, local 2P control ownership, shared-camera disagreements and disconnect/rejoin if possible.
- Mix of ordinary enemies, Boiler env interactions, hazards, Juicer and mastery-room pressure.
- Playtrace on actual manual runs; ensure strict verify/replay feasibility, record game-feel notes separately.

**Deliverable:** versioned, owner-authored or owner-accepted baseline with PASS / NEEDS TUNE / BLOCKED values per mechanical category, known acceptable tuning ranges and untested features. **If absent, do NOT make speculative global changes to shared combat/party/camera to satisfy Floor 02.** Pure-data and art work may continue.

## Staged production gates

| Gate | Outcome / definition | Allowed while owner away |
| --- | --- | --- |
| F02-G0 — Design ready | bible, art keys/kit contracts, confirmed chronology/adaptation tags, room graph, work ownership, chosen first playable room | yes |
| F02-G1 — Engine-compatible | S00 loaded from real scene route; documented v1 stage contract, checkpoint/reset, story gating, 640x360/400 and Playtrace probe | only after baseline for shared-core changes |
| F02-G2 — Combat proof | S01/S02 direct fights and one physical prop chain; scripted tests + 1x game-speed capture | yes if F02-G1 passes |
| F02-G3 — Midfloor proof | Krakaren direct+prop boss, bounded ecology, S04 and S05 convoy with solo/AI safety | yes, conditional on prior gates |
| F02-G4 — Climax proof | Rage refuge + flight with safe failover, Kennels, Ralph direct+optional dingo fight, Mongo story beat | yes, independent review required |
| F02-G5 — Real E2E | normal entry all units -> floor exit, safe restart, no future ability leakage, strict Playtrace, one run per party mode; hardware still pending | scripted portion only |
| F02-G6 — Owner fun acceptance | actual human 1P and 2P play, pacing, readability, game feel, performance, visual art signoff | **no: reserved for owner on return** |

**Stop-line:** If a later gate relies on unsafe unfinished earlier dependency, the orchestrator works on separated pure simulation, level source, art or QA rather than creating a hard-to-revert workaround. A partial setpiece is valid progress; falsely implying an entire floor is implemented is not.

## Stage production matrix (initial status)

| Stage | Primary new behavior | Depends on | Status |
| --- | --- | --- | --- |
| S00 Wrong Color | floor transition / visual contrast | stage loading, ENV-A | DESIGN |
| S01 Customer Complaints | psionic lane, physics queue | C1, EN-A | DESIGN |
| S02 Rev-Up | physical juggling of explosive props | S01, ENV-B | DESIGN |
| S03 Krakaren | tentacle boss with optional hazard route | KRAK-A, S02 | DESIGN |
| S04 Cleanup | bounded corpse ecology, flying Vespa, conveyor | ECO, C1 | DESIGN |
| S05 Meadow Lark | deterministic wheelchair convoy and defense stops | baseline AI/camera, CONVOY | DESIGN |
| S06 Terms and Conditions | refuge + invulnerable Rage reveal | RAGE-A, C1 | DESIGN |
| S07 Bomb Chicken | chase/decoy, safe stairwell expulsion | RAGE-B, baseline AI/camera | DESIGN |
| S08 No Dogs Allowed | kobold mounted combat and optional kennel allies | KEN, C1 | DESIGN |
| S09 Ralph | tiny fast boss, Mongo and exit beat | RALPH, MONGO, C1 | DESIGN |

No production assets or sprites are implied by a stage being listed.

## Testing matrix, references and evidence

Check each integrated stage against:
- one replayable scenario at fixed seed, a second seed variation and fresh/restart equivalence;
- a **without-optional-props** direct path where normal enemies or ordinary bosses are involved;
- real collision/launch/shove attribution; no stage-clearing cosmetic effects or infinite chains;
- attack tell and recovery visibility on 640x360 and 640x400;
- Carl-leading solo + Donut-leading solo simulated progression; actual 2P when owner available;
- safe checkpoints, camera and AI regroup behavior; no offscreen unfairness or unwinnable authored destruction;
- stage story-grant validation, no replay checkpoint reward duplication and IP-neutral data IDs;
- event/action/source hashes where Playtrace supports it, and strict replay result when possible;
- performance evidence on real GPU and gamepad **NOT RUN unless actually tested**, no guesses based on headless throughput.

**Representative priority recordings:** S02 direct vs prop-chain, S03 boss direct vs jug, S05 convoy defense, S07 missed decoy vs escape, S09 Ralph without dogs vs optional dog pay-off. Include actual game-speed motion: a spectacular kill is not convincing from still PNGs alone.

## Most important risks

| Risk | Severity | Rework prevention |
| --- | --- | --- |
| Floor 01 combat/companion still being tuned | critical | owner gameplay baseline, reuse before change |
| F01 E2E route/story schema still incomplete | critical | avoid building a parallel loader; C0 audit first |
| Overdesigned new floor systems | high | 10 scoped units, no inventory, no new obligatory inputs, no third human |
| Rage becomes a poor instant-kill chase | high | fair warnings, checkpoint, fail-safe escape, real 1P/2P camera tests |
| Ralph unreadable due to real small size | high | visual cue envelope, real-speed evidence and human validation |
| Environmental gimmick replaces combat | high | direct route every ordinary encounter, both bosses |
| Corpse grubs flood physics or punish stylish kills | high | bounded cluster actors, cap and CPU test, nonpunitive rules |
| Unoriginal/unauthorized IP assets | critical | source provenance, fan/noncommercial scope, independent art review |
| Parallel agents rework the same Rust files | high | strict single-owner hot file queues; short integration PRs |
| Red CI or silent replay corruption | high | full final-head local checks, strict trace, honest known gaps |
| 2P bugs and real handheld performance | high | cannot claim automated tests equal a physical pad/Steam Deck test |

## Initial suggested week-away work budget

Do not equate this list with a dated delivery commitment. Aim for: (a) reusable C0/C1 contracts; (b) reliable S00–S02 playable rooms; (c) separate Kraken, Rage and Ralph pure simulation, room data and keys; (d) original environment kit, art contact sheets, and validation harness; (e) if the earlier gates pass, implement S03–S09 and the full story route. Preserve honest blockers. Long-lived source changes should have documented migration ownership.

## Owner return decision packet

Provide a single changelog/evidence index mapping:
1. actual merged PR links and source head;
2. stage status and exact launch commands;
3. 3–5 most impressive game-speed Bevy clips plus 1x contact sheets;
4. reproducible Playtrace verification and direct/environment alternative test;
5. regression list against Floor 01 owner baseline;
6. prioritized game-feel playtest checklist and open questions.

**Final acceptance remains with the owner.** Auto-merge permission under existing project rules is not evidence that game feel, narrative art, hardware 2P or legal release are cleared.
