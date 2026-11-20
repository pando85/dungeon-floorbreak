# Agent work order: COMBAT-FUN-02 (post DEMO-01)

**Status:** design handoff only. No runtime source, art source, atlas, controller binding, tuning table or authoritative simulation was changed by this design PR. The [design proposals](../01-design/combat-fun-02-post-demo01-proposals.md) are candidate experiments; this work order organizes implementation **after design acceptance or explicit orchestration**, not a request for six agents to edit the same Bevy module immediately.

**Goal:** deliver one compact, replayable combat-focused microarena in which the owner enjoys basic attacks, enemy collisions, ground advantage, Donut participation and authored environmental kills significantly more than in DEMO-01, with clear original-pixel deaths and credible evasive choices. Do not ship a second long horizontal systems corridor.

## Read first, in this order

1. `AGENTS.md`, `docs/07-agents/workorders.md`, `docs/07-agents/agent-orchestration.md`, then the [owner playtest](../04-production/demo-01-playtest-2026-11-20.md) and [design proposals](../01-design/combat-fun-02-post-demo01-proposals.md).
2. `docs/01-design/combat-first-encounter-design.md`, `combat-feel-hybrid-physics.md`, `mvp-controller-contract.md`, `character-progression-horizon.md` and `antihero-tone.md`; do not supersede approved controls or chronology.
3. `docs/04-production/demo-01-plan.md` (B1–B6), `docs/03-tech/floor01-scene.md` and `docs/01-design/party-2p-open-questions.md`, focusing only on the relevant sections.
4. `docs/02-art/donut-moveset-q10.md`, `docs/03-tech/death-d-finishes-spec.md`, `docs/03-tech/cinematic-finishers-cine-01.md`, `tools/art-references/death-lab/README.md`, `docs/03-tech/monster-death-system-proposal.md` for their respective owners.
5. Current `main` tree and open PRs **#123** (Donut Q10/mana) and **#118** (audience HUD) before starting. Do not rely on this document's point-in-time status after branches move.

## Decisions from the owner versus candidates

**Owner evidence:** 3/5 basic combat, 1/5 enemy impacts, weak gore intensity at 1/5 (too soft), 2/5 AI and solo swap, 2/5 tone; wants more attack/combination choices, meaningful ground finishers/stomps and Donut claws, evasive options, richer rooms and a falling block that can hurt Carl. Checkpoint and 2P not tested; decap and explosive finishes not seen; automatic zoom not seen and camera sometimes jolts.

**Not approved merely by this workorder:** specific stomp input mapping, more lethal collision damage, a new evade button, auto-fighting Donut, any frame timing/damage values, larger corpse retention, widened automatic camera policy, or changes to the CINE-01 skill eligibility. For each open choice, collect evidence for at least two feasible variants and record a short recommendation with a question ID. Per AGENTS.md, the orchestrator may adopt documented design defaults when proceeding, but must not present them as owner-tested game feel.

## Stage 0 — Baseline, tooling and attribution (integrator; medium tier)

- Confirm current `main` SHA and #123 status; do not duplicate its Q10 swat/pounce/mana work. Get real source of each problem before calling it a defect: CP-02 event fire vs visual contact; death cause recorded vs off-camera/unreadable; B4 llama death class; camera swap pan, kick and zoom; Donut AI cannot initiate attacks by current design.
- Preserve DEMO-01 fixtures and goldens as regression references. Add a new `cf02_*` namespace for *proposed* fixtures or dedicated probe executable; do not silently change existing replay semantics.
- One scorecard template containing commit, branch, output size, pad ownership, mode, active story unlocks, gore preset, camera flag, binary command, per-beat observations, fps methodology and accessibility settings.
- **Gate 0:** produce one short factual gap report from logs and existing captures. No speculation promoted to an observed finding.

## Stage 1 — Combat domain A / B, separately owned (medium tier)

**CF02-A — short combo/ground-state choices**

- Own the *pure* combat-move domain and unit tests; avoid Bevy adapters during comparison. Preserve jab, haymaker and kick behavior under legacy mode and ordinary goblin time-to-kill. Experiment with one hit-confirm fork and one ground-target contextual finish using existing physical/primary verbs.
- Compare deliberate Down+Physical stomp versus proximity-context stomp; run human legibility/input conflict probes, and disallow non-deterministic automatic target grabs. Donut physical actions are from #123, not recreated here.
- Replay: whiff versus connect, press buffer, early/late second input, hit during swap, grounded vs airborne, downed target, boss immunity, multiple targets, 2P simultaneous hits, missed stomp/ordinary kick.
- **Gate A:** two distinct worthwhile follow-up branches and a ground finisher can be demonstrated without a new mandatory button or stun-lock; baseline tests stay valid or receive documented intentional migration.

**CF02-B — bounded contact follow-ups**

- Own the contact resolver and a narrowly defined event envelope, not a second collision implementation. Reuse `GoblinContact` / existing launch instance IDs and finish attribution.
- Test stagger-only versus conditional high-momentum knockdown; one effect per pair/launch, deterministic ordering, no iterative auto-damage, safe separation and walls. The visual adapter should be independent of collision physics.
- Replay: first/second actor contact, stationary target, no launch, re-entry, downed body, dead/cosmetic fragment, prop edge, two enemies on same tick, rapid restart and co-op kill credit.
- **Gate B:** 640x360 capture visibly communicates a kick-launch, target impact and readable follow-up opportunity without relying on console/HUD copy.

**Integration rule:** a single integrator serializes changes to `src/combat.rs`, `src/combat_scene.rs`, `src/goblin.rs` and event/hash definitions. Pure changes can be developed concurrently only where file ownership is disjoint; do not merge conflicting simulations blind.

## Stage 2 — Donut and swaps (medium tier; depends on #123)

- Merge/review the ongoing #123 in its existing branch/process, not in COMBAT-FUN-02. It already includes swat/pounce, the biscuit mana flag, Magic Missile and current AI still starting no attacks.
- Build an *opt-in* follower-combat A/B test: existing passive variant versus restrained nearby protective swat and exposed-target pounce variant; no off-screen combat, unsolicited last-hit stealing, magic before unlocking or negative P2 agency.
- Compare the existing solo swap timing/pan with a single faster safe preset; preserve input buffering, invulnerability and P2 ownership. Make capture of the controlled hero/AI identity and camera motion part of evidence.
- **Gate C:** the owner can tell what Donut contributed; compare kill/interrupt attribution and number of accidentally stolen kills, in solo and with two physical pads. No claim of 2P validation from synthetic fixture only.

## Stage 3 — Death and original-audio clarity (art/audio medium tier; top-tier design/review where art is new)

- **First diagnose:** scripted B3 edge decap, B5 QA blast, ordinary heavy kick and both llama kill modes. Separate missing cause, off-camera shot, poor contrast, incorrect animation pose and inaudible/missing cue. This should be a short evidence PR before reauthoring large art.
- Candidate effects: cause-specific contact and aftermath poses, bigger silhouette changes and visibly separated parts **only where species anatomy permits**; original SFX for contacts/block/crash/blast/ordinary punch; retain current corpse cap and stain quantity as baseline.
- Follow **the existing death-lab reference and pixel-art pipeline**: top-tier artist produces hard new keys/spec; medium-tier exports original editable .ase/grid sources, validates palette/pivots/foot contact and captures 1x/3x/Deck; separate top-tier review. Do not recycle art from reference games and do not copy conceptual Javascript fragments into canonical assets.
- Preserve death class, damage, event attribution, seed, bounded debris pool, presentation-only gore level and CINE-01 eligibility; reduced gore must not remove gameplay-readable confirmation.
- **Gate D:** blind review identifies three different finish causes correctly at game scale; original audio cue differentiated; no event/hash drift across cosmetic settings.

## Stage 4 — Defensive-choice experiment and compact level (levels and integrator)

- **CF02-E:** start with existing movement, explicit llama tell improvements, multiple geometry-based counters and Donut mobility, with no required new input; try a context evade only if still needed and keep it out of the main contract until approved.
- **CF02-F:** create one 40–60 second room from approved ENV-01/02 components, not a procedural/full-floor replacement. A low brawl lane, high reachable Donut perch, hard-wall ricochet zone, breakable support with obvious warning and non-softlocking protective geometry are enough. Make the load dangerous to Carl *only* with fair telegraph; settle co-op hazard attribution separately.
- Test three approaches: brute force; environment; swap/2P coordination. Same protected exit and same meaningful choices in all legal prop states. Do not show void above the widest camera view or at 640x400.
- **Gate E/F:** one player can explain the upcoming hazard without coaching; two legal avoidance routes exist for llama spit; environment has at least one verifiable offensive consequence, solo and 2P stay completable.

## Stage 5 — Integration and owner game-feel acceptance

**One integrated build** using an explicit `--cf02` or similarly named isolated opt-in scenario until accepted. Do not replace `--demo`, early story progression or existing fixtures with untested variants.

- Automated: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`, `bash scripts/ci_floor01_combat.sh headless`, existing HUD/Bevy render checks, asset validators for new art, newly committed fixed-step replays; run the checks against the final rebased head.
- Regression: exact existing hashes stay unchanged unless a gameplay rule *intentionally* changes; then record old/new goldens and a migration note. Equal outcomes under gore 1/4, cinematic on/off and camera choices. No unbounded contact recursion, unexpected timing dependence, offscreen hazard hits or AI-only kill requirements.
- Evidence: real Bevy captures (640x360, 640x400 and 1920x1080), brief GIF at actual 1x plus contact sheets, 1P swap and **real two-pad** recordings, camera view traces and performance/frame budget on an actual target GPU/Deck-class device. If real pads unavailable mark manual test **NOT RUN**, do not substitute CI assertions.
- Ask the owner to replay the same room three times, once per approach. Review baseline vs after scores in the playtest file; do not declare 'fun achieved' based on passing tests. The owner can reject any tuning after merge.

## PR decomposition and review budget

| PR | Primary owner | Changed scope | Dependency |
| --- | --- | --- | --- |
| CF02-00 | integrator / QA | baseline gap report, objective traces, new independent fixtures | main / #123 status |
| CF02-A | combat domain | combo and downed branch A/B; small new state only | CF02-00 |
| CF02-B | combat domain | launch/contact consequence and proof | CF02-A event merge or stable independent contact API |
| CF02-C | party | opt-in Donut follower A/B and swap feel | #123, CF02-00 |
| CF02-D | art + audio | death cause clarity, authored native parts, event-driven SFX | CF02-B event frozen |
| CF02-E | movement/gameplay | evasive counters without input-contract rewrite | baseline and llama probes |
| CF02-F | levels + integrator | one rich microarena; only then integrate the other branches | CF02-A–E |
| CF02-REVIEW | integrator / QA | scorecard, evidence, regressions and owner handoff | all gates |

Route medium tier for implementation, top tier for genuinely unsettled attack/animation design and independent review. Do not spend top-tier calls regenerating screenshots/CI evidence. End each PR with status, checks, evidence links, known untested human cases and next dependent PR. Gameplay and Bevy merges follow the existing AGENTS.md delegated review rules; owner game feel remains separate.

## Required report (each agent)

1. Feature/variant and **visible** result, including an A/B baseline.
2. Branch, head SHA and any authoritative events/inputs changed.
3. Commands actually executed and CI status; never claim unrun controller tests.
4. Before/after deterministic hashes and any deliberate migration.
5. Original pixel/audio evidence, if affected.
6. Remaining risks and the next integration owner.
7. Specific answer: **why is this more fun, at 1x, without adding a new mandatory button?**

**Rights:** use only already-authorized original project sources; no copied franchise or other game's art/music/sounds. Commercial distribution rights remain unresolved. **This document requests no new images or ZIP intake.**
