# COMPANION-AI-02 — Donut autonomy: implementation work order

**Status:** design proposal, Revision 2 (2026-11-21), not started. Defaults CA2-Q01 to CA2-Q35 are adopted by the orchestrator (owner design-defaults directive, 2026-11-19; owner decisions of 2026-11-21 in spec section 0.1); game feel is pending the owner's play. **CA2-A is implemented** (zone, drift check, claim set, engagement predicate and every CA2 knob as data: `src/companion/autonomy.rs`, `src/companion/engage.rs`, the `autonomy` block and the `cf02_free` set in `companion_ai_v0.json`; nothing is wired, no golden moved); **CA2-B1 is implemented** (the goal selector `AutonomyState::step`, spot sampling over a `SpotWorld`, the director's `step_briefed` and `GoalBrief`, `NavDriver::go_to` and `NavSpots`, `CompanionEvent::GoalChanged`; still unwired, no golden moved; B1 ran alone, B2 is not small and disjoint). B1 adopted defaults: AD-B1 the front claim counts a lying or staggered enemy only when it is down from the leader's own hit (review N-5, FU-D); AD-B2 equal utilities go to Escort, then Support, then Hunt (Escort 800 plus the travel bonus 400 ties a new Hunt at 1200, and she comes along); AD-B3 Show-off starts within 45 ticks of her landed hit, a code constant (the `smug` overlay window); AD-B4 Escort is a candidate only when it has work (the leader travels, or she is past the comfort radius), otherwise she holds as Idle, because an always-eligible Escort at 800 outranks Show-off (700) and Perch (600) in every quiet moment and neither could ever be chosen (a design note for the top tier). Perch is selected from CA2-C. CA2-B2 onward is not started. Notes from the implementation: `leader_engages(set, leader, target)` takes no `view` (the predicate reads only the leader and the target); the corridor accessor is `SceneSim::camera_pan_remaining()` (`pub`, so it is not dead code before CA2-B2 reads it); the shared knob `ca2_q12_carl_low_hp` is named `ca2_q12_leader_low_hp`; the Hunt base and travel terms of spec 4.2 are role-table data (`ca2_q14_base_*`, `ca2_q14_travel_divisor`); the missile's attacker hit-stop is 0 (the feel table's 2 ticks freeze the target only). The finding-to-fix map of the review is in [spec section 9](../03-tech/companion-ai-02-donut-autonomy.md#9-revisions) (Revision 1 and the re-review's Revision 2).

**Spec:** [COMPANION-AI-02 — Donut autonomy inside the camera](../03-tech/companion-ai-02-donut-autonomy.md). **Read also:** [COMPANION-AI-01 work order](companion-ai-01-workorder.md) ("Shared files and who owns them when", "Cross-stage regression suite", "PR description and review handoff"), [companion spec](../03-tech/companion-ai-command-system.md) sections 8 and 17 (e), [orchestration](agent-orchestration.md) ("Standard brief", "Speed mode", "Temporary review gate"), [AGENTS.md](../../AGENTS.md), and the DONUT-MOBILITY-01 spec and work order (design PR #173; `docs/03-tech/donut-mobility-01.md` and `docs/07-agents/donut-mobility-01-workorder.md` once merged).

## Mission

Make the AI Donut act on her own judgement: her own spacing, her own targets, perches, retreats and small show-offs, within one hard limit: she stays inside the current camera view, and no camera ever zooms, widens or moves for her. Build the layer role-neutral (leader and partner, a role goal table in data) and ship only Donut's `skirmisher` table. Deliver it behind the new profile set `cf02_free` (`--companion cf02_free`), in small mergeable PRs, with every existing golden byte-identical.

## Routing

- **Design:** done by the top tier (this file and the spec). Design problems found during execution go back to the top tier with a short note; the executor does not re-decide them.
- **Execution:** medium tier, one agent per PR below.
- **Review:** an independent reviewer per PR against the spec sections the PR names (top tier; under the [temporary review gate](agent-orchestration.md#temporary-review-gate-owner-decision-2026-11-20) a separate medium-tier agent, and the PR then owes a top-tier re-review after the reset). The owner judges game feel after merge.

## Prerequisites: PR #170 and DM-A

**PR #170 is merged** (`62686919`): A11 to A15, `TargetView.carl_hit_age`, the `steals` and `start_steals` counters. What CA2 does with it:

| #170 item | In CA2 |
| --- | --- |
| A08, A11, A12 (who Carl engages) | Kept as the hard veto; their engagement test is extracted once as `leader_engages` (CA2-A) and is the core of the leader's claim set (spec 4.2) |
| A13 (turn to a target behind her) | Kept |
| A14 (same-tick known limit) | Kept, still pinned by its non-golden test |
| A15 (flank spot 56 px ahead of Carl) | Kept in `cf02_assist`; **superseded** in `cf02_free` (`cf02_duo_a15_flank` false, CA2-Q28). Its private `flat_span` is generalised into the ungated `flat_span_at` for spot validation; `CompanionView.flank_span` is not used |
| `steals`, `start_steals` | Reused as metric M4 (must be 0) |

**DONUT-MOBILITY-01 stage DM-A must be merged before CA2-A starts.** CA2 depends on its `HeroMotion` (the leader's run speed in the drift check), on `NavConfig::agile()` and its preset selection in `navfollow.rs`, and it shares `corridor.rs`, `party.rs`, `floor01_scene.rs`, `playtrace/replay.rs` and `cf02_fixtures.rs` with it. By owner decision (2026-11-21) Donut's agile motion is her default everywhere, so DM-A also sets the golden baseline CA2 must keep byte-identical. DM-B (evasion: biscuit unlock, reflex mode only, 720-tick (12 s) rearm) touches no companion file and may run alongside any CA2 stage; CA2 never reads the evasion state.

## Shared files and owners

One owner at a time. Before starting, run `gh pr list` and check that no open PR touches these files; a PR that finds a file owned by another running PR stops and reports instead of editing it (COMBAT-LAB-03, DONUT-MOBILITY-01 and any 01D work are the likely neighbours).

| File | Sequence |
| --- | --- |
| `src/companion/autonomy.rs` (new) | A -> B1 -> C |
| `src/companion/engage.rs` (new: `leader_engages`) | A only |
| `src/companion/mod.rs` | A (two lines) |
| `src/companion/profile.rs`, `assets/data/gameplay/companion_ai_v0.json` | A only (all CA2 knobs land at once; later PRs change values only through a recorded adopted default) |
| `src/companion/director.rs` | A (extract the `leader_engages` call from `steals_the_finish`, no behaviour change) -> B1 (`step_briefed`, `GoalBrief`) |
| `src/companion/navfollow.rs` | DM-A (preset selection) -> B1 (go-to entry) -> C (hazard input to `NavDriver::new` / `sync`) |
| `src/companion/events.rs` | B1 -> C |
| `src/combat_scene/companion.rs` | B2 (the `step_briefed` call site, `flat_span_at`, wiring) -> C |
| `src/companion_fixtures.rs`, `src/cf02_fixtures.rs` | (DM-A) -> B2 -> C |
| `src/corridor.rs` | (DM-A) -> A (one read-only `pub(crate)` accessor for the scroll camera's remaining pan; `Corridor::pan_boosted()` already exists and is reused) |
| `src/party.rs` | (DM-A) -> B2 (the `cf02_free` profile choice, if the party enum names profile sets) |
| `src/bin/floor01_scene.rs` | (DM-A) -> B2 (`--companion cf02_free` parse) -> D (optional debug overlay) |
| `src/scene_camera.rs` | B2 only (CA2-Q25: the partner moves to `bound_only`) |
| `src/camera_lab/camera_op.rs`, `src/camera_lab/session.rs` | B2 only (`CameraInput::bound_only`, empty for every existing caller) |
| `src/playtrace/replay.rs` | (DM-A) -> B2: a definite edit, `Some("cf02_free") => UncontrolledHero::Companion(ProfileSetId::Cf02Free)` in `live_tuning` |
| `docs/evidence/companion-ai-02/**` | B2 -> C -> D |
| `docs/03-tech/companion-ai-02-donut-autonomy.md` | status lines and measured numbers only, by each PR; design changes go back to the top tier |

## PR sequence

Speed mode allows grouping B1 with B2, and C with D, when the diff stays reviewable. Each grouped PR keeps the acceptance criteria and fixtures of every stage it contains.

### CA2-A — Zone core, engagement predicate and data (no behaviour change)

- **Goal:** the pure free-zone, drift-check and claim-set code and every CA2 knob exist and are tested; nothing is wired.
- **Change:** `src/companion/engage.rs` with `leader_engages(view, set, leader, target)`, extracted from `CompanionDirector::steals_the_finish` (A12, A11, A08 engagement; the gates and HP reserve stay in `steals_the_finish`, which now calls it); `src/companion/autonomy.rs` with `FreeZone::compute` (spec 3.1, both vertical limits), `DriftCheck` (3.3, the leader's run speed from his `HeroMotion`, worst-case ticks), `LeaderClaims` (4.2); the `autonomy` profile block in every set (`enabled` false in `companion_v0` and `cf02_assist`), the role table `autonomy.roles.skirmisher`, `autonomy_role` per hero block (Donut `skirmisher`, Carl `none`); the new set `cf02_free` (= `cf02_assist` with `enabled` true and A15 off); `ProfileSetId::Cf02Free`; profile validation; the `corridor.rs` read-only accessor.
- **Tests:** `trail_cap` equals `view_w / 2 + DEADZONE_PX - LEAD_PX` and holds against the real `ScrollCamera` (the leader starts at every offset inside the deadzone, facing either way, runs away for 300 ticks: the distance from him to the trailing edge never drops below `min(initial, 336)`), under both the legacy and the agile motion; zone shrink near a view edge and at the layout ends; the rise limit; the worst-case commitment sums pinned against the move-table constants (pounce 54, swat 16, missile 16 plus its attacker hit-stop) and the drift check for each, plus a nav transit from an edge's simulated duration; `leader_engages` equals the old inline predicate on a fixed set of views (and every `steals_the_finish` result is unchanged); the claim set against A08, A11, A12 and the 64 px front claim, including its expiry after the A12 window once the leader is more than 64 px away; profile pin updated (`PROFILE_HASH` moves, a data pin only); the CA2-Q25 bound pad is at least `ceil(max partner run px per tick / PAN_RATE) + 4` for every partner motion (36 with DM-A; the adopted 40 passes).
- **Acceptance:** every golden byte-identical (no wiring); `cargo fmt`, `clippy -D warnings`, `cargo test`, `bash scripts/ci_floor01_combat.sh headless`, `python3 scripts/check_docs.py` pass.

### CA2-B1 — Goal selector and director brief (pure, no wiring)

- **Goal:** the goal layer decides Escort, Hunt, Support-leader, Retreat and Idle with spot choice and hysteresis (spec 5.1), the director accepts an optional brief that gates it (5.2), and the state follows the lifecycle (5.4).
- **Change:** `autonomy.rs` (`AutonomyState` with cooldowns as absolute ticks, `decide`, spot sampling every 16 px on safe anchors in the zone, the brief's `allowed` set and press band per goal, the recoil window, the boss fallback CA2-Q35, `hash_into`, `clear_plan` and `reset`); `director.rs` `step_briefed(view, set, brief: Option<&GoalBrief>)` with `step_traced` kept as the `None` wrapper: with a brief, `best_support` and `revalidate` (every tick) accept only targets in `allowed`, the director writes no `move_x` or turn for a support plan and presses only inside both bands; an interrupt keeps its approach; `navfollow.rs` go-to entry (the `follow` logic toward a spot, no 40 px wait band, committed transits and closure handling reused); `events.rs` `GoalChanged` (and the reason) for traces.
- **Tests:** a fixed set of views gives byte-identical director output through `step_traced` and through `step_briefed(None)` (pin it); goal eligibility and utilities per CA2-Q18; one goblin in `C` gives no Hunt goal and no press on it (review B-2); after a landed pounce the director emits no `move_x` for the 30 recoil ticks and in Retreat and Show-off (review B-3); a plan dies on the first tick its target leaves `allowed`; the Retreat trigger (a hurt only, the low-health radius, the shared cooldown); Show-off ignores lying and staggered enemies; hysteresis (CA2-Q16, Q30); immediate triggers (interrupt, Retreat, leaving the zone); the support rules (4.3, a claimed target only when the leader is low and no unclaimed one is close); the lifecycle table (swap round trip with equal hashes for equal inputs, respawn, reset); every director veto still applies under a brief; no `HashMap`, no RNG.
- **Acceptance:** every golden byte-identical.

### CA2-B2 — Wiring, camera rule and the first fixtures (behaviour under `cf02_free` only)

- **Goal:** `--companion cf02_free` plays the autonomous Donut; every other mode is unchanged.
- **Change:** `combat_scene/companion.rs`: under `cf02_free` the goal steering replaces the follow bot for the partner (dodge first, then the director's approach only for an interrupt, then the goal; goal steering and the interrupt approach clamped to `Z`, dodge and committed transits exempt); `step_briefed` at the partner's call site, `None` for AI Carl; `flat_span_at` as a pure helper beside `flat_span`; the autonomy state hashed only while she is companion-driven under `cf02_free`; metric counters. `party.rs`, `floor01_scene.rs` and `playtrace/replay.rs`: the `cf02_free` choice. `camera_op.rs` (and `session.rs` literals): `CameraInput::bound_only` with the spec 3.4 semantics (Revision 2: her bound clamps the **total** threat-driven shift of the pan target relative to the leader-only centre `tx0`, never-worse, at the narrower of the current and the transit-target width, after the leader bias clamp and before the room-edge clamp; `target_cy` computes the leader-only result first and applies her range only when it intersects, so no fallback re-centres on her); `scene_camera.rs`: the partner moves to `bound_only` under `cf02_free` in solo while she is companion-driven (CA2-Q25).
- **Fixtures (goldens, replays committed, recorded with DM-A's motion):** `cf02_free_explore_room`, `cf02_free_split_two_goblins`, `cf02_free_lone_claimed`, `cf02_free_finish_abandoned`, `cf02_free_outnumbered`, `cf02_free_carl_bolts`, `cf02_free_retreat_when_hurt`, `cf02_free_arena_direct`. None contains a hero-hurting hazard (the hazard feed lands in C; the PR body lists it as a known gap). Metrics harness `autonomy_metrics()` (spec 7.1, M5's exact predicate) beside `offence()`.
- **Non-golden tests:** the bolt sweep (spec 7.2) under both motions; M7 (scroll camera reads the controlled body only; automatic-camera rest-width series (`rest_target()`, `zoom_changes`) unchanged when her position moves by ±200 px; the left series keeps her body inside the view); the named R-1 test `bound_only_llama_opposite_640`: 1280-wide output (rest width 640), Carl at C, Donut at C - 240, a llama aiming at Carl at about C + 300 on the other side, in five phases (the llama admitted to the framing box: the target settles at about C + 24, not C + 155.6; she runs away from Carl at her DM-A speed with the bound active; the same run while an 8-tick `impact_hold` is set (the I-1 pace rule keeps the normal pace); the llama leaves the framing set and the target steps back to `tx0`; the mirror image), her body inside the view on every tick, and the same series without her clamp putting her off screen (so the test guards the case); a `target_cy` case where her range does not intersect the leader's and the leader-only value is kept; M8 probe at 640 and 960 rest widths, both aspects.
- **Evidence:** GIFs 1, 2, 3 and 6 of spec 7.3 under `docs/evidence/companion-ai-02/` with `SHA256SUMS`; a PR-body table of M1 to M16 for each fixture, with the `cf02_assist` value on the same inputs beside M1, M2 and M13.
- **Acceptance:** M3 to M9 and M12 at target; M1, M2, M13 and M15 at target or the shortfall reported honestly with the measured numbers (a shortfall is a design note for the top tier, not a reason to retune silently); every non-`cf02_free` golden byte-identical.

### CA2-C — Perch, Show-off, hazards and the llama

- **Goal:** the remaining goals and the safety feed.
- **Change:** Perch (CA2-Q23, including Donut-only ledges the agile nav preset opens, inside `Z` and both vertical limits) and Show-off (CA2-Q24; stand still so CF02-I's existing `smug` overlay plays, no presentation change and no new art); hazard rectangles that can hurt a hero fed into the live nav graph (first verify which scene hazards can hurt a hero and list them in the PR body; if none can, add the plumbing with an empty list and a test, and say so); spot risk, ledge inset and enemy-lane penalty (CA2-Q29); the llama rules (CA2-Q31, 4.2).
- **Fixtures:** `cf02_free_perch_and_show_off`, `cf02_free_hazard_edge`, `cf02_free_llama_biscuit`, `cf02_free_llama_prebiscuit`. `cf02_free_explore_room` may move (perching); the PR lists old and new hashes with the reason.
- **Evidence:** GIFs 4 and 5.
- **Acceptance:** M9, M10, M11 at target; M4 to M6 still 0; every non-`cf02_free` golden byte-identical.

### CA2-D — Evidence pack and owner handoff

- **Goal:** the owner can play it and judge it from one page.
- **Change:** `docs/evidence/companion-ai-02/README.md`: how to launch (`--cf02 --companion cf02_free`, once more with `--donut-biscuit`), what to look for (her own target, the hit and run, finishing a goblin Carl left, the perch, the sulk, the smug hold, never off-screen while Carl runs, and that `--camera auto` still widens for threats but never for her), the adopted-defaults table with links to the spec, the measured metric table, and the questions for the owner (spec section 8). Optional presentation-only overlay under `--debug-combat` that draws the free zone and the goal label, and still 7 of spec 7.3. CPU cost of the autonomy layer (M14), measured, not estimated.
- **Acceptance:** `check_docs.py` passes; evidence checksums match; NOT RUN items (human play, real pad) listed plainly.

## Brief template for each executor

- **Goal:** the PR's goal line above.
- **Read:** the spec sections the PR names, this file's PR section and "Shared files and owners", the merged #170 diff of `director.rs` and `view.rs`, and the merged DM-A diff of `prototype.rs`, `nav.rs` and `navfollow.rs`.
- **Constraints:** no edits to `docs/04-production/backlog.md` (speed mode); no new move, input, unlock or art; no RNG outside the seeded simulation; no companion-only speed or damage; no new teleport; no utility that reads the evasion state; every existing golden byte-identical except those the PR lists with a reason; `None` brief and `enabled` false mean today's behaviour byte for byte; code contracts say leader and partner, never a character name.
- **Acceptance:** the PR's acceptance list; the baseline commands of the [COMPANION-AI-01 regression suite](companion-ai-01-workorder.md#cross-stage-regression-suite).
- **PR body:** Why, Change, Acceptance Criteria (checked only when verified), Commands & Results, Visual Evidence, Performance Data, Rights/Provenance (no art change; GIF frames are committed original art under the recorded scope; which model executed), Risks, adopted defaults, horizon-fit line, Next PR.
- **Report:** about 15 lines: status, PR number, head SHA, CI state, blockers, open questions, paths.

## Risks

- **She could feel passive** with a single goblin that Carl works (she leaves it to him by design, and Revision 1 drops the post-biscuit poke). M13 and M15 guard against it; the owner decides whether the claim rule is too strict (CA2-Q11 is data).
- **Pre-biscuit pounces are short-ranged** from the leader: the worst-case drift (54 ticks) allows a landing within about 117 px of him. Goblins that walk at him come into range; isolated ones far away wait. The lever is the leash or the comfort radius, not removing the check.
- **Goblins retarget to her** more often once she stands apart (Q3). That is an intended side effect; Retreat bounds it. Report the count of goblins that targeted her per fixture (M16), and the share of Hunt candidates the drift check rejected beside those the claim set excluded (M16, re-review FU-A), so the owner and the top tier can see which rule makes her passive; the cheapest lever is CA2-Q03.
- **The drift check is conservative** (it assumes the leader runs away for the whole commitment and gives her no speed credit). That keeps the "every exit is involuntary" rule exact.
- **The automatic camera** holds her only through the bound-only rule. If M8 fails at some rest width or aspect, the levers are the bound pad, the rise limit (CA2-Q08) or the leash (CA2-Q04), in that order, never making her a framing subject again (which would let her widen the camera).
- **DM-A slips or changes** its numbers: CA2-A waits for it; the zone and drift rules take the motion as data, so only the measured numbers move.
- **Narrow-check items (spec 9.3), in B2:** the I-1 pace rule in `step_pan` (normal pace while her bound holds or her body plus pad is outside the view; `urgent` wins; no-op with `bound_only` empty); the apex-aware rise limit (no jump or hop edge planned from a surface more than about 137 px above the leader's feet, rechecked per tick).
- **Re-review follow-ups fixed forward** (spec 9.2): FU-C, the `cf02_free_finish_abandoned` goblin may chase Carl back inside 64 px or come at his back before she presses; B2 pins it (for example keeps it down) or asserts "not in `C` at the press tick and at least 32 ticks after his last hit" instead of the 64 px distance. FU-D, the 64 px front claim counts only a standing enemy, so a goblin Carl knocked down 41 to 64 px in front of him can be finished by her while he walks up; B1 settles whether lying and staggered enemies count, as a design note back to the top tier.
