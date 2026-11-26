# F01-CHASE-DIRECTOR-01: local agent work order

**Task:** implement the Floor 01 introductory Murder Dozer chase as a cinematic but **fully player-controlled** set piece, following [the choreography](../01-design/floor-01-dozer-opening-choreography.md). The numbers, contracts, pins and the PR split are in [the Act I integration plan](../01-design/floor-01-act-01-integration.md). This document is the design and work-order start; it is not a finished gameplay feature.

**Provenance.** This work order comes from the owner's draft PR #179. That PR stays open and unmerged as the record of the original text. This copy keeps the draft's guardrails and acceptance questions. It changes three things:

- the deliverables map onto the two ACT1 PRs;
- the conflicts list reflects current main;
- the 8 s fail-open policy is replaced by the commit rule of integration plan section 5.3.

## Read and branch coordination

Follow `AGENTS.md`, `docs/07-agents/workorders.md`, [the orchestration guide](agent-orchestration.md) and the merged code **before touching any shared module**.

Audit:

- `docs/02-art/env-02-opening-spec.md`, `assets/levels/floor01/opening_route_proposal.json` and `docs/02-art/doz-01-dozer-spec.md`;
- room 3's machine: `src/dozer.rs`, `src/combat_scene/dozer.rs`, `src/f01_dozer.rs`, `src/encounter/haz.rs` and `assets/data/gameplay/enemy_dozer_v0.json`;
- the run: `src/f01_late.rs`, `src/f01_run.rs`, `assets/levels/f01_late/manifest.json`;
- `src/bin/floor01_scene.rs`.

Reuse the original source art and animation (the approved Dozer rig, the ENV-02 timber and blocker) at their exact native sizes and markers.

**Coordination on current main:**

- **Merged, no conflict.** PR #174 (Carl progression), #175 (Donut progression), #176 (Goblin Boiler) and #178 (Hoarder, Dozer revenge, Slime-Rot) are merged. The Dozer revenge is room 3. The opening **reuses its machine** through a second profile; it does not fork it. Room 3's data, fixtures and goldens must stay byte-identical.
- **Companion movement.** Companion movement and navigation are owned separately. The opening extends the existing solo rescue with two appended spots only (integration plan section 5.5).
- **REST-WIRE PR 2** (camp and aftermath) is merged. ACT1-A and ACT1-B re-pin the manifest and the pin lists from its values.
- **Do not silently rewrite** Playtrace, the render replay, CF02 or the camera.

## Deliverables, in dependency order

| Subtask | Deliverable | Evidence / pass condition | ACT1 PR |
| --- | --- | --- | --- |
| CHDIR-A | ENV-02 **option R**: the real S1 to S5 geometry and markers as playable run rooms with two characters | At-speed Bevy captures at 640×360 and 640×400, both party modes; no new level-loader fork (the rooms use the room and sidecar formats) | ACT1-B (`act1_chase`), ACT1-A (`act1_shore`) |
| CHDIR-B | The DOZ-01 h120 rig and fixed-step positional pursuit; the S2 entry; two one-shot S3 timber events; wheel collision; the one-tile humps | Headless state and hitbox traces; real animation at 1x; death and retry from x 1008 | ACT1-B |
| CHDIR-C | The visible committed charge warning in S4; the shared refuge at x 2064 to 2160; barrier collision and destruction; an honest wedge and overshoot in S5 | The beat order follows real contact; the exit and the goblin fight are reachable; the camera stays consistent | ACT1-B, ACT1-A |
| CHDIR-D | **Co-op fairness**: x 1888 **arms** the charge; the commit waits until every living hero can safely reach the pocket, including at 448 px separation. **Adapted:** no fail-open timer; with no charge, the cruise opens the barricade, so there is no softlock and no forced lethal charge | A deterministic two-character feasibility matrix (ACT1-F13, T11); real two-pad runs; a fixed-seed repro for any failing case | ACT1-B |
| CHDIR-E | Presentation: two splinter beats, the close wheel pass, the crash, the optional unintrusive failure hint, native VFX. **Deferred:** the music intro and ducking, the engine and whistle (no runtime audio, ACT1-Q2) | No presentation signal changes collision, the event log or the state hash; presentation on and off give the same gameplay (ACT1-F17) | ACT1-B |
| CHDIR-F | Test fixtures and recordings: CHDIR-01 to 12, the restart after the chase, recovery into the S5 goblins | `playtrace verify` for the three rooms; at least three at-speed captures (a first failure, a safe co-op near miss, a complete exit); the owner's game-feel sign-off stays **pending** | Both |

## Technical and art guardrails

- **Only change** the existing machine, room adapters and reusable gameplay components that are needed. A charge sub-state, a pocket rule, a lock event and a small set of appended events are fine. A general vehicle AI, a new generic cutscene system, a movie-style overlay or a new movement verb are not.
- **No HP or damage frames** for the opening Dozer. Standard attacks spark against the hull but never defeat or stun it. Carl cannot leap the wheel with the existing physics. The moment is **survival only**; the Dozer revenge is room 3.
- **Never replace player input** with a noninteractive scripted path. Trigger timing may be authored and deterministic. Player motion, the refuge, the hitboxes, the wheel contact and the barrier break all run through fixed-step domain state.
- **The barricade.** The full-height variant is authoritative; the deprecated short background variant is not a lane blocker. The real blocker vanishes in the tick of the Dozer's contact (heroes collide with the open state from the next tick, integration plan section 5.4), never because a cutscene reaches a set second.
- **The end-on wedge** through the S5 arch (a visible pose at most 192 px wide) is a production art constraint. Until it exists, the greybox clips and darkens the side-on rig and is labelled as greybox (ACT1-Q3). ENV-02's wall-breach contingency needs owner review.
- **The companion's pathfinding** is owned elsewhere. In solo, manual swapping must not kill the unselected companion on every run, and the rescue spots keep the bot out of the wheel. 2P may fairly fail for bad decisions but must never be mathematically doomed by ordinary screen separation.
- **No invented first-floor content:** no unlocks, special input, dynamite, mana, new enemy, viewers or sponsor elements. No cinematic slow motion: exceptional kill rewards stay separate from surviving this chase.
- **Data and determinism.** Every time, speed and distance exception is data-driven and tested at 60 Hz. VFX, raster, lighting and audio never change sim outcomes. Respect Playtrace's live recording defaults and the native art, provenance and rights constraints.

## Exact acceptance questions

1. **How many seconds does the S1 spawn to S5 escape really take** for novice and practiced solo players and for two real humans? Compare against the 25 to 40 s hypothesis. Do not pad the duration by inserting waits.
2. Can new players identify **the danger of the wheel** and **the right safe alcove** on their first attempt, without a long tutorial?
3. Does the charge **commit in one direction**, rather than track a player and kill them in the alcove?
4. For two humans 0, 96, 224 and 448 px apart at x 1888, does each have a geometrically possible path to the refuge before the charge reaches them? If not, fix the policy or timing, or call out that owner approval is required.
5. Do the two timber-splinter events, the 104 px low refuge, the 96 px full barricade and the wedge all render at the approved 1x scale in the real scene and respect collision?
6. Does a failure restart at x 1008 without replaying S1? Does a win reach x 2640 and continue directly into the I-03 goblin encounter? (On main this is the next run room.)
7. Do identical captured fixed-tick inputs reproduce exactly the same movement, actor ownership, damage, reset, prop states and event order, even with audio and lighting disabled?
8. Does the first goblin melee after the near miss feel like a satisfying emotional reversal? Ask for a recorded human verdict.

**Do not claim owner approval** from an automated success metric. The final PR keeps:

- the checklist;
- the actual commands and tests;
- native screenshots;
- source provenance and performance data;
- the 1P and 2P evidence;
- a short honest "not yet working" section.

**Minimum good PR outcome:** one playable S1 to S5 chase with a reachable refuge, a fully deterministic barrier and checkpoint, the correct DOZ-01 appearance and a short verified Playtrace. Audio, the end-on art and the final 2P feel may remain explicit follow-ups when they need separate asset or review ownership. **Do not** mark the feature done until the fairness blockers are resolved.
