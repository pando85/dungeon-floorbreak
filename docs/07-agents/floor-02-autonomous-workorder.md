# F02 autonomous production — local agent work order (F02-LAST-LESSON-01)

**Status:** implementation plan only; no Floor 02 code, assets or playtest is claimed by this document. Owner intention: finish Floor 01 human playtesting **before leaving**, then let a local orchestrator and delegated agents implement as much Floor 02 as safely possible during the one-week trip. **All commands and PRs are in English.**

**Sources:** [F02 game bible](../01-design/floor-02-last-lesson-design-bible.md), [visual bible](../02-art/floor-02-concrete-hell-visual-direction.md), [F02 roadmap](../04-production/floor-02-production-roadmap.md). Read and obey AGENTS.md, agent-orchestration.md, ADR-0001, ADR-0009, ADR-0010, ADR-0011, local-validation/speed-mode merge policy, the current Floor 01 end-to-end PR and current test/backlog state. **Never assume a design-only Floor 01 PR is implemented.**


## Second-level briefs: REQUIRED reading before assigning executable tasks

**The ten beat IDs S00–S09 are five contiguous player-facing arcade chapters, not ten separate result-screen levels.** Reuse the stage boundary and chapter checkpoint semantics in the design bible. The agent must not implement ten disconnected launchable fixtures and declare Floor 02 complete.

| Workstream | Mandatory dedicated specification | Stop gate |
| --- | --- | --- |
| Rev-Up + Krakaren | [F02-RK-01](../01-design/floor-02-revup-krakaren-setpieces.md) | direct and spectacle S02, then direct and prop S03, real 1x evidence |
| Meadow + Rage | [F02-MR-01](../01-design/floor-02-meadow-rage-setpieces.md) | deterministic convoy, verified sanctuary, no-cart escape, then optional Bomb Chicken |
| Kennel + Ralph + Mongo | [F02-RALPH-01](../01-design/floor-02-ralph-kennel-finale.md) | tiny boss physical hit/read test FIRST, direct victory, then optional dog and original art |

**Delegation boundary:** these design documents specify the *what*, the candidate gameplay hypotheses and acceptance. The orchestrator may choose implementation approach based on inspected current code, documenting every deviation and baseline effect. Owner's Floor 01 human feel is the shared-core gate; pure state machines and authored data do not wait for an invented approval.

## Additional design briefs are prerequisites to implementing the associated work

- [F02-FOUNDATIONS-01](../01-design/floor-02-arrival-cleanup-encounters.md): S00/S01/S04 greybox, Mind Horror, capped grubs and conveyor.
- [F02-ACT-01](../01-design/floor-02-enemy-acting-bible.md): personalities/poses and production art/animation acceptance.
- [F02-DIRECTOR-01](../01-design/floor-02-director-and-continuity.md): chapter staging, checkpoints, viewer attribution and **true Floor 03 transition only AFTER S09**.

**P0 implementation trap:** Rage-only S07 stairwell expulsion **must not advance the playable party to Floor 03**. Make separate level-data / event IDs for Exile(Rage) and ExitFloor02(party after S09), and add a deterministic test before integrating the chase. If "foreman" appears in any Rev-Up document, cast it as the **Laminak elite consultant gameplay role**, not a new unrelated species.

## 0. Preconditions and explicit HOLD points

1. **Record repository head**, open PRs and actual implementation state (not titles), then identify shared hot files and ongoing Floor 01 orchestration. Avoid competing with its file queue.
2. **Human Floor 01 gameplay baseline**: require an owner-authored or owner-accepted GAMEPLAY-BASELINE-V1 with controller behavior, combat feel, camera, solo companion, two-pad usability, known high-risk problems and decisions before merging any Floor 02 code that changes shared gameplay. If unavailable, independently advance docs, level source data, art keys, pure isolated enemy/boss state machines and tests; mark integrated gameplay **BLOCKED ON BASELINE**.
3. Read the approved Floor 01 core state, not only the older README. Use real existing scene loader, enemy slot, story unlock, env prop, hazard, party, camera and Playtrace contracts where possible. **No duplicate simulation, camera, input or actor-control subsystem.**
4. Legal/IP, asset provenance, accepted ADR conflict, nonreversible architecture, human game feel and genuine two-pad play are **human gates**. Do not fabricate acceptance or circumvent these during absence.
5. Existing Floor 01 must remain playable; all novel floors are opt-in until validated, with tests on the final PR head. Do not change shared simulation silently to fit F02 content. Record mechanical deltas for owner inspection.
6. **Source verification**: secondary fan summaries are not authoritative book prose. Before finalizing the on-screen narrative story beats, verify order against primary novel if legitimately accessible; otherwise mark the beat as game adaptation or uncertain. Never copy text/quotes/third-party assets.

## 1. Deliverables and ownership

Primary orchestrator owns architecture contract, PR sequencing, integrator merges, risk assessment, independent review coordination, batch backlog updates and owner return report. Do not set every agent to edit src/bin/floor01_scene.rs, combat_scene.rs, assets/levels manifest or docs/04-production/backlog.md simultaneously. Allocate exactly one owner per shared file per batch, separate worktrees and lean external target dirs; see orchestration rules.

Role lanes:
- **A: engine & data integrator** — inspect/extend existing stage manifest/loader, story progression, level validator, checkpoint, runtime stage switching, solo/camera input owners. Minimal isolated changes.
- **B: Floor 02 gameplay** — new per-enemy state machines, environmental props, boss fixtures, authored room source data. No unsolicited shared hot-file edits.
- **C: visual art** — original source keys and repeatable kits, sprite atlases + manifests, 640x360/400 game-scale previews, independent art review.
- **D: QA/replay** — room seeds, direct-vs-spectacle fixtures, autoplay coverage, Playtrace, restart/camera/2P diagnostics, performance and truthfulness of DONE labels.

**Definition of an integrated stage**: real scene loaded on the current build with both hero representations and enemy/props; actual collisions and interactive outcomes; deterministic headless fixture; reproducible retry; screenshot/GIF from actual renderer; strict replay where possible; direct path demonstrated; no impossible story grants; status **merged, human game-feel pending** (NOT approved).

## 2. Execution plan, dependency graph and PR-ready acceptance

### F02-C0 — prerequisite contract audit (design-only, no hot files)

Produce a concise architecture map comparing existing Floor 01 rooms / expected E2E route against Floor 02 needs. Classify each need as reused as-is, extension with tests, or unimplemented. Identify current CLI room-launch commands and tests **from actual code**, not guessed flags. Output a reusable floor/stage authoring contract with **versioned IDs** and migration constraints. Stage definition: id, route predecessor/successor, environment kit, enemy roster, actor/prop IDs, checkpoint/respawn, spawn and camera bounds, available story flags, exit condition, coop regroup policy and evidence ids. Preserve LDtk source-of-truth intent where supported; do not force a premature whole-level format migration.

Acceptance: no changes to gameplay; contract proves Floor 01 data can still be parsed or has an explicit migration test. Repo documentation points to actual existing types and owners.

### F02-C1 — minimal reusable floor-stage path (depends C0 + owner baseline for shared-core changes)

Implement one Floor 02 entrance-to-exit toy room using existing engine/party/camera/Playtrace without editing any Floor 01 golden. Add data-driven stage select and a story-safe checkpoint/transition pathway ONLY as needed. One direct and one optional actor-route proof. Introduce no new mandatory input. Feature flag / room select keeps prior default unchanged.

Acceptance: headless/load/quit/retry and real capture; no broken Floor 01 baseline; both protagonist slot configurations possible. Do not claim human co-op passed.

### F02-ENV-A + F02-ENV-B — original environment kits (parallel to C0/C1)

ENV-A: F02-ENV-01 cinderblocks, lichen, corridors/doorframe, lights, reflective walking-line validation; ENV-B: sales floor jugs/rack/booth, refuse conveyor, convoy props, stairwell, kennel and ring keys. Produce native editable masters, atlas/metadata, provenance and 1x/2x scene proof under the global pipeline. Use independent top-tier art review. Agent may use a labelled stand-in where production art is not yet ready. **Never** import screenshots or fan art as sprite data.

### F02-EN-A — basic enemy families (pure modules first, after C0)

Implement Mind Horror psionic lane, Clurichaun slingshot and foreman alarm with enemy-slot conventions. 1-step defensive/counter opportunities and no infinite reinforcement. Expose tests for attack tells, direct physical hits, knockback, solo partner target rules and deterministic resets. Only then wire to S01/S02.

### F02-S00-S02 — first three rooms (depends C1 + EN-A + environment stand-ins)

Create S00/S01/S02 source data, walkable geometry, setpiece and two exit variants. Reuse Carl/Donut pre-existing capabilities from the owner-approved baseline. S02 props must yield actual physics chain outcomes and a complete no-prop direct route. Record one exact-seed PLAYTRACE, a gameplay-speed GIF and the deterministic contact log. Stop scaling until the **S02 direct and spectacle routes** both pass integration tests.

### F02-KRAK-A/B — first real boss and aftermath (depends EN-A + S02)

A: pure Krakaren Clone state machine and fixture with 3 attack modes, recovery windows, boss damage, scoped weak body and tested direct/prop win. B: real S03 room render, camera, telegraph, optional environment and capped grub transition. A large sprite is NOT an excuse for always-visible offscreen attacks; ensure both native resolutions.

### F02-ECO — corpse ecology + S04 (depends C1 + KRAK-A)

Build a deterministic, bounded grub-cluster system (pool events derived from authoritative defeats), one Vespa airborne attack and a conveyor/cage hazard reusing existing environmental actor contracts. Test max entities, bounded work, replay stability and no kill-tax death spiral. Integrate S04 after pure tests.

### F02-CONVOY — Meadow Lark moving refuge, S05 (depends C1 + baseline companion/camera)

Author 3–4 visually linked wheelchair segments along a **scripted scene track with two stops**. It cannot be a generic physics-cart escort requiring NPC pathfinding. Enemy waves are bounded and telegraphed; no farmable civilian harm. Dual-hero camera and AI recovery must be validated. Deliver a quiet checkpoint.

### F02-RAGE-A/B — the exceptionally dangerous setpiece, S06/S07 (depends C1 + baseline)

A: pure invulnerable hazard controller, protected refuge, state machine, mandatory message, fair recovery, scripted decoy, alternate bypass, exit trigger, correct defeat attribution (NO XP). B: real camera/companion-safe room and escape, stairwell, evidence and retry. **No normal boss HP**. The Rage is expelled by the stage rules rather than killed by damage. If AI follow or camera cannot safely guarantee S07, stop integration and finish isolated tests/design; do not sneak in broad global camera changes.

### F02-KEN — kennel and optional ally flags, S08 (depends EN-A + C1)

Implement kobold rider/mount separation as an authoritative collision outcome and a local kennel flag via existing contextual interact. Test with all cages closed, rescued dingo states and no mandatory ally damage. No global pet system.

### F02-RALPH — final boss, S09 (depends KEN + C1)

Pure state machine first: three distinguishable rhythms, small physical silhouette with broad *visual tell*, real fair hurtbox, direct physical/magic victory and optional dingo swallowing event that **does not finish the boss**. Prove no forced Carl kill and no one-button 2P marginalization. Then wire ring/presentation. Validate local geometry and frame capture against approved native sizes.

### F02-MONGO — narrative epilogue (depends RALPH)

Re-use existing approved Mongo concept identity for original native source art and story flag. Hatchling/wary/recognition keys; one quiet room, proper checkpoint and exit. No third actor controls, 3P, mount or unapproved combat follower. Do not introduce class/race selection on Floor 2.

### F02-E2E + QA — connect and verify (depends ALL game-critical tracks)

A single intact normal-entry -> S09 -> Floor 3 stairwell journey, no independent isolated-room teleports disguised as actual floor flow. One complete deterministic scripted run + room-level independent tests; solo lead both ways, AI partner + 2P scripted and real 2P NOT RUN where hardware missing; real game-speed footage, screenshots, Playtrace strict verify and restart proof for each significant encounter. Event-by-event floor transition ledger and room acceptance matrix. Artifact paths and reproduction commands recorded in owner packet.

## 3. Work queue and merge policy during the trip

**Suggested first wave:** F02-C0, ENV-A, pure EN-A, original S09 arena key art; these do not require shared gameplay assumptions. **Second wave:** C1 -> S00–S02 -> Krakaren A/B; ECO and convoy can progress on pure isolated data. **Third wave:** Rage A/B and Kennels/Ralph; Mongo art independent. **Final wave:** E2E, perf, evidence and top-tier review backlog.

Approve reversible local defaults (timing windows, enemy counts, prop positions) as data with stable question IDs F02-OQxx, not hidden engine constants. If the owner baseline reveals different action timing or partner behavior, re-pin fixtures with documented causal explanation; do not rewrite core combat from Floor 02. No single agent may merge a change requiring new IP/license rights, a contradictory ADR, mandatory new controller verbs, speculative minion system or irreversible engine migration.

Current orchestration rules allow independent reviews, local-validation speed-mode merge and batch backlog updates, but **none bypass human game-feel approval**. If the review-tier quota gate is in effect, document substitute review debts. PR summary format: Behavior, adopted defaults, source/evidence, tests on final SHA, performance, rights/provenance, known gaps, game-feel pending, next PR.

Required local validation, run where applicable on the final head:
- cargo fmt --all -- --check
- cargo clippy --locked --all-targets -- -D warnings
- cargo test --locked --all-targets --no-fail-fast
- python3 scripts/check_docs.py
- bash scripts/ci_floor01_combat.sh headless
- Art pipeline tests and export checks per AGENTS.md if any art changes.

**No speculative PR titles should be treated as code delivery.** When integration cannot be verified, mark BLOCKED/NOT RUN, leave intact files, and advance non-overlapping tasks.

## 4. Return packet for the owner

One short index document with:
- branch + merged PR numbers and last tested SHA; changed core systems and migration notes;
- stage matrix S00–S09: DESIGNED / PURE SIM / INTEGRATED / AUTOPLAY / REAL HUMAN PLAY / NEEDS FIX, each backed by paths;
- one real 640x360 or 640x400 scene GIF per major setpiece, a 1x contact sheet and a controller-speed record;
- pass/failed/retry Playtrace results and a precise executable launch command for each available stage;
- direct-versus-spectacular kill facts, AI participation counts, estimated time per stage (NOT human data), profiler runs;
- grouped **owner game-feel questions** plus highest priority fixes;
- a single recommended 20-minute return playtest path, followed by optional full-floor run.

Never represent a headless bot clear as entertainment validation. The owner's pending playtest is the release gate.
