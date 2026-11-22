# F02-EXEC-01 — One-week autonomous execution packet and work queue

**Status:** actionable work specification, not code or a promise that features are implemented. 2026-11-22. Owner plans Floor 01 hands-on testing before 2026-11-25 departure, then local-agent implementation while away. Every execution agent must read AGENTS.md and current agent orchestration rules. Design PR #239 remains a draft until the local docs check and independent review.

**Authority/read order for this packet:** owner-accepted Floor 01 GAMEPLAY-BASELINE-V1 (when available) -> repository architecture/ADRs -> [verified engine mapping](../03-tech/floor-02-engine-reuse-and-route-contract.md) -> [Floor02 game bible](../01-design/floor-02-last-lesson-design-bible.md) -> [six encounter/acting/director briefs](../01-design/floor-02-encounter-composition-and-difficulty.md) -> [QA guide](../03-tech/floor-02-automated-encounter-evaluation.md). Follow source references named in each work item rather than reading 10 large docs for every small issue.

## Shared campaign integration reference (F03 PR #246)

**One cross-floor story owner:** the [CAMPAIGN-BRIDGE-01 test matrix](https://github.com/pando85/dungeon-floorbreak/blob/design/f03-over-city-bible/docs/03-tech/floor-03-campaign-bridge-acceptance.md) in the separate **draft PR #246** defines the expected Floor01→02→03→04 capability carry, F02 S07 Rage-only stair, true post-Ralph/Mongo Floor03 entry, Floor03 Guild class narrative, early circus stair and final Remex/Floor04 exit. This is a **proposal and 20 test vectors, not an implemented engine API**. The Floor02 integrator must coordinate one route/story authority with the future Floor03 author and should not create incompatible floor-specific plot ledgers. Floor02's playable S00–S03 slice remains first in priority.

## 0. Product goal and anti-rework agreement

Deliver **as much real, coherent Floor 02 gameplay as the actual current architecture safely supports**, keeping Floor 01 tests and user-approved game feel stable. Five player-facing continuous chapters A–E from ten internal encounter beats S00–S09. The Floor 02 should have satisfying direct physical combat; spectacle routes are optional and physical; the Rage Elemental escape is a specifically marked exception to normal HP combat; Ralph is genuinely tiny and still fair; only after Ralph/Mongo does the party descend to Floor 3.

**Best outcome:** an actual entrance-to-exit floor with evidence and a short owner return playtest plan. **Acceptable fallback:** S00–S03 real integrated combat vertical slice, independently validated pure gameplay states and complete original asset/level data for later beats, with honest blockers. **Bad outcome:** many merged scene stubs, no connected traversal, untested mechanic changes to F01, an auto bot declaring the game fun or a giant implementation PR unable to isolate regressions.

No changes to the four permanent controller verbs, no inventory/gear grind, no new third human/actor controls, no mandatory "Second Chance" summons, no baked-in lore copyrighted speech/audio/art. All asset production uses original source and provenance with the required editor review. No public playable distribution based solely on concept-art publication permission.

## 1. The human baseline is a contract, not an agent permission gate for isolated work

**Before departure:** owner plays Floor 01 and accepts a compact `GAMEPLAY-BASELINE-V1` (see companion owner template). Agent should ingest its recorded revision and exact tested SHA, classify fixed rules / tuning objections / known gaps. Never invent the baseline or infer game feel from a green test.

If absent:
- **ALLOWED:** independent docs/research, original art keys and masters subject to provenance, stage geometry in separate source files, pure enemy/boss state machines, fixture and QA development, read-only architecture mapping, compatible parser-only work that proves no F01 behavior change.
- **HOLD:** changes to shared attack timing, native scale, input layout, AI companion behavior, 2P camera, generalized combat physics and replacing Floor 01 progress/camera/loader authority. A breaking change needs an accepted ADR / owner review, not a silent refactor.
- Don't repeatedly request owner authorization while away; build reversible work without a shared-core conflict and write a concrete one-page BLOCKED note.

## 2. Shared-file allocation

Assign exactly ONE integrator for shared hot files: `src/lib.rs`, `src/bin/floor01_scene.rs` + `src/bin/floor01_scene/*`, `src/encounter.rs`, `src/encounter/haz.rs`, `src/combat_scene.rs`, `src/story_unlock.rs`, `src/playtrace/replay.rs`, `src/party_scene.rs` and shared hashes/manifests. Other agents work in unique new `src/f02_*` modules, `assets/levels/floor02/*`, art source dirs and evidence dirs. One code change owner owns the final merge queue and rebase. Never overwrite active Floor 01 PR or `docs/04-production/backlog.md` from multiple branches; orchestrator alone posts batch summaries.

Parallel lanes:
- **I — Integration/architecture:** route/loader, versioned story, room run, one launcher, replay pins and merges; medium implementation, top-tier on architecture conflict.
- **G — Gameplay:** pure role AIs, prop states, bosses, room blueprints; medium execution, top-tier review for new shared semantics.
- **A — Art/sound:** original concrete kit and enemy key art, native animations, export, checksums, original SFX; route by current silhouette/asset score and independent review.
- **Q — QA:** scripted policy adapters, negative fixtures, report generator, performance, strict replay, real 1x capture; medium implementation, independent reviewer on failures.

## 3. Dependency-oriented tasks

Every task has a narrow observable result, references and a stop criterion. In the implementation backlog use these stable IDs, NOT unapproved issue numbers.

| ID | Priority | Depends on | One complete deliverable | Hard acceptance / stop |
| --- | --- | --- | --- | --- |
| F02-W00 audit | P0 | none | current SHA, in-flight PR map, actual F01/party/renderer/replay interfaces and exclusive file ownership | no guessed APIs or CLI; documents only |
| F02-W01 game contract | P0 | W00, owner baseline for shared changes | backward-compatible route reader or minimal F02 adapter, one stage transition and checkpoints | F01 CLI + golden pass; no duplicate world |
| F02-W02 environment kit | P0 | art bible, independent of W01 | original sterile concrete floor/wall/lichen, signage and one functional hit/prop visual, with source masters | approved hero legibility at actual native size, provenance/pivots |
| F02-W03 Mind Horror | P0 | W00 | pure enemy with targeted psychic line, committed attack and reachable recovery; standalone test | no input inversion, both heroes viable |
| F02-W04 Clurichaun / Laminak | P0 | W00 | two enemy roles: actual slingshot + alarm, one finite reinforcement and real interrupts | ≤ existing roster budget, no repeated helper spawn |
| F02-W05 sales props | P0 | W00 | jug/rack/falling sign through existing stimulus/colliders, root damage cause and valid ignition | direct fight still works; plain Magic Missile does not ignite by fiat |
| F02-W06 rooms S00–S02 | P0 | W01/02/03/04/05 | original greybox, real Bevy load, S00→S01→S02 physical direct and optional route | S02 direct + chain + missed prop replay; native capture |
| F02-W07 Krakaren | P0 | W05/W06 | rooted boss S03, 3 telegraphed attacks, exposed core, direct and prop-based victory | Carl and Donut direct; strict replay; real game-speed clip |
| F02-W08 ecology cleanup | P1 | W01 and boss transition schema | finite grub clusters + Vespa + moving conveyor and suspended bin in S04 | no uncapped spawn/perf blowup; one direct route |
| F02-W09 Meadow convoy | P1 | W01, Floor 01 companion/camera baseline | authored deterministic 2-stop protected wheelchair convoy S05 | solo and two-slot tests; no offscreen NPC loss, hardware pending |
| F02-W10 Rage state | P1 | W01, story transition and party baseline | S06 refuge and S07 invulnerable pursuer/stair exile with no-cart alternative | no HP/XP, sanctuary protection, party remains F02 |
| F02-W11 kennel | P1 | W01 | rider/dingo state split, real speaker impact and optional local kennel aid S08 | all-cages-closed direct win, detached ownership correct |
| F02-W12 tiny Ralph | P1 | W01/W11 | boss S09, native small hit/read test, direct Carl/Donut clear and optional dog beat | no invisible huge hitbox, dog never auto-kills |
| F02-W13 Mongo | P1 | W12, approved concept refs | small original narrative-only hatchling, quiet outro and exit event | no third actor controls, only true post-Ralph Floor03 transition |
| F02-W14 linked run | P1 | W06–13, F01 bridge when available | continuous five chapters, story loadout, checkpoint/resume and strict replay | no teleport-only fake E2E, real skip/retry provenance |
| F02-W15 evidence pack | P0 continuous | each landed scene | per-stage commands, native 1x game-speed capture, bot scorecard and known gaps | truth table with NOT RUN human/pads |
| F02-W16 independent review | P0 per batch | any merge candidate | fresh top-tier functional/art/rights check and minimal fix list | objective blocking faults closed before merge |

**P0 work is a priority, not a promise of completion on a certain day.** Orchestrator may independently schedule W08/W10/W12 pure simulation early if no shared-file access is needed. For all significant new systems implement **pure logic and tests before Bevy integration**, then replay and real rendering. If W01 cannot safely generalize, prefer compatible isolated scene adapters rather than forcing a major refactor.

## 4. First implementation vertical slice: specific desired demo

**Four beats S00–S03.** The minimum owner-return demo should show:
1. Crossing the new white-concrete seam under player control into a one-family introductory fight.
2. Mind Horror announces a psychic line, Carl/Donut can evade and punish without new buttons.
3. Rev-Up Clurichaun and Laminak fight; one complete route kills by direct attacks **with sales props untouched**.
4. An independent attempt kicks an enemy into a real rack; authored jug rolls, legitimate second stimulus ignites it, a falling sign actually collides with a survivor; failed chain can recover via standard combat.
5. Krakaren Clone reveals a readable weak body and three committed move tells, and is defeated once without a prop and once with a real credited prop impact.
6. Checkpoints and solo partner/2-slot routes exist; exact events are replay-verified; one **real Bevy screenshot and motion clip** at native 640×360/400 is linked.
7. No F01 accepted moves were weakened to make these encounters easier; human game feel remains PENDING.

**A local auto agent can implement hundreds of lines but cannot mark this slice done until it is both playable and replayable.** If the existing launcher refuses party modes on new rooms, it reports a clear blocker and tests underlying pure two-slot combat without claiming 2P launch is available.

## 5. Parallel work queue if integration is blocked

| Blocker | Productive safe work | Forbidden shortcut |
| --- | --- | --- |
| Waiting owner baseline | original F02 art, pure enemy/boss modules, level source data and tests, doc/asset manifests | change global attack timing or companion AI |
| F01 complete route not wired | build Floor02's isolated story-safe stages and room-to-room movement, isolated adapter | claim Floor01→Floor02 E2E exists |
| Playtrace descriptor lacks F02 | narrow per-room versioned data pin/replay adapter, fixtures, checks | rewrite replay goldens or bypass strict verification |
| Camera cannot cover Rage two heroes | pure Rage states, sanctuary geometry and authored camera-safe corridor alternatives | shrink accepted character sprites or teleport inactive partner secretly |
| Clurichaun chain bypasses fun | direct enemy role tuning and separate ignition/prop tests | arbitrarily nerf Donut magic or add HP |
| Ralph too small to kick | adjust local boss posture, collision *policy* and visual tell, compare native 1x | enlarge a hidden hurtbox or globally alter hero kick |
| Asset quality poor | original keys, independent silhouette review, placeholder with explicit provenance | ship concept illustration resized to pixel texture |
| CI or local checks fail | isolate/fix reproducible local cause, report exact failing command/SHA | report green or overwrite another active PR |

## 6. PR sequencing and integration gates

Use small numbered local PRs, but **do not pre-allocate actual GitHub PR numbers**. Suggested feature PR names: `f02-route-contract`, `f02-enemy-core`, `f02-sales-physics`, `f02-first-slice`, `f02-krakaren`, `f02-ecology`, `f02-convoy`, `f02-rage`, `f02-kennel-ralph`, `f02-story-e2e`, `f02-evidence`. Rebase independently and verify each final head. Merge authoring/data and pure modules freely under current local review policy only when existing invariants hold. Human-only rights/canon/irreversible architecture gates remain unresolved unless owner provided actual acceptance.

**GATE 0 — Documentation:** design PR reviewed, merged or explicitly treated as provisional source; exact chronological doubts documented.
**GATE 1 — Safe engine:** one F02 real room loads; F01 old commands/goldens pass.
**GATE 2 — First fun candidate:** S02+S03 direct and optional routes complete and evidence captured, independent review passes.
**GATE 3 — Remaining content:** S04–S09 works individually, boss and chase have no softlocks.
**GATE 4 — Linked floor:** chapters connect from story loadout, scene carry/retry works, 1P and scripted 2-slot replay verifies.
**GATE 5 — Human acceptance:** human actual 1P+2P feel / real pad / performance on return; cannot be auto-approved.

## 7. Exact check protocol (run locally, report status truthfully)

Examples from current agent instructions; the local agent must identify actual profile flags and test scripts from HEAD rather than claiming these ran in this document.

~~~sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
bash scripts/ci_floor01_combat.sh headless
python3 scripts/validate_assets.py   # when art/assets change
~~~

Add actual new F02 tests and one strict PLAYTRACE verification for each meaningful room. Reuse F01 scorecard infrastructure only where its data contract allows; do not silently rename an F01 scorecard and claim comparable results. Render must use Bevy scene output; AI-generated concept art is **not** a test screenshot.

## 8. Owner return packet: one view, not 100 PR notifications

Orchestrator publishes a single `docs/evidence/f02-return/README.md` or equivalent index with:
- baseline revision and exact current tested main commit, dated list of merged PRs and reverts;
- ten-beat matrix showing DESIGN / PURE / INTEGRATED / AUTOPLAY / STRICT REPLAY / 1x RENDER / HUMAN for each, without collapsing statuses;
- direct and optional props modes, exact CLI launch or NOT AVAILABLE, failed tests with reproducible command;
- 3–5 *actual rendered* game-speed snippets: S02 chain, S03 boss, S05 convoy, S07 stair exile, S09 Ralph; where missing, clearly list not rendered;
- real companion attack counts and AI path failures, true S07 vs S09 story exit check;
- p95/p99 real hardware FPS if measured; distinguish headless/Xvfb performance, NEVER invent Steam Deck result;
- one 20-minute owner playtest route with target questions, actual shared-core deviations, saved traces and the prioritized top five fixes.

The owner's vacation does not authorize sending private recordings, chat, account data or unreviewed book assets to remote services. All large ML/playtrace logs remain local unless the current privacy settings authorize disclosure.

## 9. Ready-to-use orchestrator kickoff prompt

> Use PR #239 as the provisional design authority for Floor 02 and follow AGENTS.md. First complete F02-W00, compare latest main and open gameplay PRs, identify ownership of shared hot files and the exact owner Floor 01 GAMEPLAY-BASELINE-V1. Then run F02-W01–W07 as the critical vertical slice, parallelizing original art, pure enemy/boss logic and negative QA tests. Never refactor core combat/party/camera without the baseline or explicit review; preserve all Floor 01 goldens. Each merged unit needs reproducible real simulation tests and game-rendered evidence where applicable. If one integration gate fails, progress on independent W08–W13 pure modules/assets and report the blocker honestly. Do not mark human fun, real two-pad or rights approval from automated results. Keep one concise F02 return packet with exact commands, PR links, snapshots and top owner game-feel questions. Work to completion under existing local review/merge autonomy.

**This is an implementation brief, not evidence of any delivered game content.**
