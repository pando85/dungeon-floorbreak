# COMPANION-AI-02 — Donut autonomy: implementation work order

**Status:** design proposal, not started. Defaults CA2-Q01 to CA2-Q32 are adopted by the orchestrator (owner design-defaults directive, 2026-11-19); game feel is pending the owner's play. Nothing here is implemented.

**Spec:** [COMPANION-AI-02 — Donut autonomy inside the camera](../03-tech/companion-ai-02-donut-autonomy.md). **Read also:** [COMPANION-AI-01 work order](companion-ai-01-workorder.md) ("Shared files and who owns them when", "Cross-stage regression suite", "PR description and review handoff"), [companion spec](../03-tech/companion-ai-command-system.md) sections 8 and 17 (e), [orchestration](agent-orchestration.md) ("Standard brief", "Speed mode", "Temporary review gate"), [AGENTS.md](../../AGENTS.md).

## Mission

Make the AI Donut act on her own judgement: her own spacing, her own targets, perches, retreats and small show-offs, within one hard limit: she stays inside the current camera view, and no camera ever zooms, widens or moves for her. Deliver it behind the new profile set `cf02_free` (`--companion cf02_free`), in small mergeable PRs, with every existing golden byte-identical.

## Routing

- **Design:** done by the top tier (this file and the spec). Design problems found during execution go back to the top tier with a short note; the executor does not re-decide them.
- **Execution:** medium tier, one agent per PR below.
- **Review:** an independent reviewer per PR against the spec sections the PR names (top tier; under the [temporary review gate](agent-orchestration.md#temporary-review-gate-owner-decision-2026-11-20) a separate medium-tier agent, and the PR then owes a top-tier re-review after the reset). The owner judges game feel after merge.

## Prerequisite: PR #170

CA2 builds on #170 (A11 to A15, `TargetView.carl_hit_age`, `CompanionView.flank_span`, the `steals` and `start_steals` counters). **No CA2 PR touches a companion file until #170 is merged.** What CA2 does with it:

| #170 item | In CA2 |
| --- | --- |
| A08, A11, A12 (who Carl engages) | Kept as the hard veto and reused as the core of the claim set (spec 4.2) |
| A13 (turn to a target behind her) | Kept |
| A14 (same-tick known limit) | Kept, still pinned by its non-golden test |
| A15 (flank spot 56 px ahead of Carl) | Kept in `cf02_assist`; **superseded** in `cf02_free` (`cf02_duo_a15_flank` false, CA2-Q28). Its `flank_span` helper is reused to validate spots |
| `steals`, `start_steals` | Reused as metric M4 (must be 0) |

If #170 changes before it merges, the executor of CA2-A reads the merged version and records any difference in its PR body.

## Shared files and owners

One owner at a time. Before starting, run `gh pr list` and check that no open PR touches these files; a PR that finds a file owned by another running PR stops and reports instead of editing it (COMBAT-LAB-03 and any 01D work are the likely neighbours).

| File | Sequence |
| --- | --- |
| `src/companion/autonomy.rs` (new) | A -> B1 -> C |
| `src/companion/mod.rs` | A (one line) |
| `src/companion/profile.rs`, `assets/data/gameplay/companion_ai_v0.json` | A only (all CA2 knobs land at once; later PRs change values only through a recorded adopted default) |
| `src/companion/director.rs` | B1 only (optional `GoalBrief`) |
| `src/companion/navfollow.rs` | B1 (go-to entry) -> C (hazard input to `NavDriver::new` / `sync`) |
| `src/companion/events.rs` | B1 -> C |
| `src/combat_scene/companion.rs` | B2 -> C |
| `src/companion_fixtures.rs`, `src/cf02_fixtures.rs` | B2 -> C |
| `src/corridor.rs` | A (one read-only `pub(crate)` accessor: the scroll camera's remaining pan and the solo swap pan flag) |
| `src/party.rs` | B2 (the `cf02_free` profile choice, if the party enum names profile sets) |
| `src/bin/floor01_scene.rs` | B2 (`--companion cf02_free`) -> D (optional debug overlay) |
| `src/scene_camera.rs` | B2 only (CA2-Q25 subject rule) |
| `src/playtrace/**` | B2, only if the session header enumerates companion choices (verify; do not edit otherwise) |
| `docs/evidence/companion-ai-02/**` | B2 -> C -> D |
| `docs/03-tech/companion-ai-02-donut-autonomy.md` | status lines and measured numbers only, by each PR; design changes go back to the top tier |

## PR sequence

Speed mode allows grouping B1 with B2, and C with D, when the diff stays reviewable. Each grouped PR keeps the acceptance criteria and fixtures of every stage it contains.

### CA2-A — Zone core and data (no behaviour change)

- **Goal:** the pure free-zone, drift-check and claim-set code and every CA2 knob exist and are tested; nothing is wired.
- **Change:** `src/companion/autonomy.rs` with `FreeZone::compute` (spec 3.1), the drift check (3.3), the claim set (4.2) and the vertical band; `DonutAutonomy` profile block in every set (`enabled` false in `companion_v0` and `cf02_assist`) and the new set `cf02_free` (= `cf02_assist` with `enabled` true and A15 off); `ProfileSetId::Cf02Free`; profile validation; the `corridor.rs` read-only accessor.
- **Tests:** `trail_cap` equals `view_w / 2 + DEADZONE_PX - LEAD_PX` and holds against the real `ScrollCamera` (Carl starts at every offset inside the deadzone, facing either way, runs away for 300 ticks: the distance from Carl to the trailing edge never drops below `min(initial, 336)`); zone shrink near a view edge and at the layout ends; drift check for pounce, swat, missile and a nav transit; claim set against A08, A11, A12 and the 64 px front claim; profile pin updated (`PROFILE_HASH` moves, a data pin only).
- **Acceptance:** every golden byte-identical (no wiring); `cargo fmt`, `clippy -D warnings`, `cargo test`, `bash scripts/ci_floor01_combat.sh headless`, `python3 scripts/check_docs.py` pass.

### CA2-B1 — Goal selector and director brief (pure, no wiring)

- **Goal:** the goal layer decides Escort, Hunt, Support-Carl, Retreat and Idle with spot choice and hysteresis (spec 5.1), and the director accepts an optional brief.
- **Change:** `autonomy.rs` (`AutonomyState`, `decide`, spot sampling every 16 px on safe anchors in the zone, `hash_into`); `director.rs` `step_traced(view, set, brief: Option<&GoalBrief>)`, with `None` exactly today's path; `navfollow.rs` go-to entry (the `follow` logic toward a spot, no 40 px wait band, committed transits and closure handling reused); `events.rs` `GoalChanged` (and the reason) for traces.
- **Tests:** a fixed set of views gives byte-identical director output with `None` (pin it); goal eligibility and utilities per CA2-Q18; hysteresis (CA2-Q16, Q30); immediate triggers (interrupt, Retreat, leaving the zone); the claim penalty and the support rules (4.3); the target policy filters director candidates while every director veto still applies; no `HashMap`, no RNG.
- **Acceptance:** every golden byte-identical.

### CA2-B2 — Wiring, camera rule and the first fixtures (behaviour under `cf02_free` only)

- **Goal:** `--cf02 --companion cf02_free` plays the autonomous Donut; every other mode is unchanged.
- **Change:** `combat_scene/companion.rs`: under `cf02_free` the goal steering replaces the follow bot for Donut (dodge first, then the director's approach, then the goal; every `move_x` clamped so it does not carry her out of the view); the autonomy state is hashed only while she is companion-driven under `cf02_free`; metric counters. `party.rs` and `floor01_scene.rs`: the `cf02_free` choice. `scene_camera.rs`: CA2-Q25 (controlled hero is the only framing subject in solo under `cf02_free`).
- **Fixtures (goldens, replays committed):** `cf02_free_explore_room`, `cf02_free_split_two_goblins`, `cf02_free_outnumbered`, `cf02_free_carl_bolts`, `cf02_free_retreat_when_hurt`, `cf02_free_arena_direct`. Metrics harness `autonomy_metrics()` (spec 7.1) beside `offence()`.
- **Non-golden tests:** the bolt sweep (spec 7.2); M7 (scroll camera reads the controlled body only; automatic camera series unchanged when her subject position moves by ±200 px); M8 probe at 640 and 960 rest widths.
- **Evidence:** GIFs 1, 2, 3 and 6 of spec 7.3 under `docs/evidence/companion-ai-02/` with `SHA256SUMS`; a PR-body table of M1 to M14 for each fixture, with the `cf02_assist` value on the same inputs beside M1, M2 and M13.
- **Acceptance:** M3 to M9 and M12 at target; M1, M2 and M13 at target or the shortfall reported honestly with the measured numbers (a shortfall is a design note for the top tier, not a reason to retune silently); every non-`cf02_free` golden byte-identical.

### CA2-C — Perch, Show-off, hazards and the llama

- **Goal:** the remaining goals and the safety feed.
- **Change:** Perch (CA2-Q23) and Show-off (CA2-Q24; stand still so CF02-I's existing `smug` overlay plays, no presentation change and no new art); hazard rectangles that can hurt a hero fed into the live nav graph (first verify which scene hazards can hurt a hero and list them in the PR body; if none can, add the plumbing with an empty list and a test, and say so); spot risk, ledge inset and enemy-lane penalty (CA2-Q29); the llama rules (CA2-Q31, 4.2).
- **Fixtures:** `cf02_free_perch_and_show_off`, `cf02_free_hazard_edge`, `cf02_free_llama_biscuit`, `cf02_free_llama_prebiscuit`. `cf02_free_explore_room` may move (perching); the PR lists old and new hashes with the reason.
- **Evidence:** GIFs 4 and 5.
- **Acceptance:** M9, M10, M11 at target; M4 to M6 still 0; every non-`cf02_free` golden byte-identical.

### CA2-D — Evidence pack and owner handoff

- **Goal:** the owner can play it and judge it from one page.
- **Change:** `docs/evidence/companion-ai-02/README.md`: how to launch (`--cf02 --companion cf02_free`, once more with `--donut-biscuit`), what to look for (her own target, the hit and run, the perch, the sulk, the smug hold, never off-screen while Carl runs), the adopted-defaults table with links to the spec, the measured metric table, and the questions for the owner (spec section 8). Optional presentation-only overlay under `--debug-combat` that draws the free zone and the goal label, and still 7 of spec 7.3. CPU cost of the autonomy layer (M14), measured, not estimated.
- **Acceptance:** `check_docs.py` passes; evidence checksums match; NOT RUN items (human play, real pad) listed plainly.

## Brief template for each executor

- **Goal:** the PR's goal line above.
- **Read:** the spec sections the PR names, this file's PR section and "Shared files and owners", and the merged #170 diff of `director.rs` and `view.rs`.
- **Constraints:** no edits to `docs/04-production/backlog.md` (speed mode); no new move, input, unlock or art; no RNG outside the seeded simulation; no companion-only speed or damage; no new teleport; every existing golden byte-identical except those the PR lists with a reason; `None` brief and `enabled` false mean today's behaviour byte for byte.
- **Acceptance:** the PR's acceptance list; the baseline commands of the [COMPANION-AI-01 regression suite](companion-ai-01-workorder.md#cross-stage-regression-suite).
- **PR body:** Why, Change, Acceptance Criteria (checked only when verified), Commands & Results, Visual Evidence, Performance Data, Rights/Provenance (no art change; GIF frames are committed original art under the recorded scope; which model executed), Risks, adopted defaults, horizon-fit line, Next PR.
- **Report:** about 15 lines: status, PR number, head SHA, CI state, blockers, open questions, paths.

## Risks

- **She could feel passive** with a single goblin that Carl works (she leaves it to him by design). M13 guards against a drop in offence; the owner decides whether the claim penalty is too strong (CA2-Q11 is data).
- **Goblins retarget to her** more often once she stands apart (Q3). That is an intended side effect; Retreat bounds it. Report the count of goblins that targeted her per fixture.
- **The drift check is conservative** (it assumes Carl runs away for the whole commitment). If it blocks too many pounces at range, the lever is the leash or the comfort radius, not removing the check.
- **The automatic camera** only frames the controlled hero under `cf02_free`; if M8 fails at some rest width, lower CA2-Q04 rather than adding her back as a subject.
