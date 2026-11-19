# CINE-01 — Skill-gated cinematic slow motion for exceptional defeats

**Status:** owner-approved *creative intent* (2026-11-19); implementation pending. This document is the implementation work order for a local coding agent. The numerical defaults below are **adopted starting hypotheses**, not owner-approved game feel. No gameplay or Bevy behavior is implemented by this documentation PR.

**Goal:** In THE CRAWL, an unusually well-executed defeat briefly makes the action feel cinematic. The player earns that moment by timing, setting up an environmental kill, or constructing a meaningful chain—not by killing an ordinary enemy or selecting a violent death recipe. The effect must be **rare**, unmistakable, and over before it interrupts the pace.

Read first: `AGENTS.md`; [combat feel](../01-design/combat-feel-hybrid-physics.md); [universal defeat language](../01-design/universal-monster-death-language-v0.md); [death architecture](monster-death-system-proposal.md); [DEATH-D](death-d-finishes-spec.md); [party design](../01-design/party-2p-open-questions.md); [camera/accessibility](input-camera.md); and [work order protocol](../07-agents/workorders.md). Respect the [long-term character horizon](../01-design/character-progression-horizon.md): this is a presentation reward for existing verbs, **not** a new move, unlock, resource, or progression ability.

## 1. Non-negotiable behavior

1. **Exceptional execution is required.** A death is necessary but not sufficient. Neither `ForceTier::Catastrophic`, gore preset 4, a dismemberment recipe, an explosion, high damage, nor a boss death automatically triggers slow motion.
2. **Causality over chance.** Use observed, authoritative combat events and verified player attribution. No random chance, visual-debris collisions, VFX counts, estimated audience/viewership, or opaque “coolness” heuristics.
3. **One shared world, one shared camera.** In solo and local 2P, both heroes, all enemies and hazards undergo the same brief time dilation. Do not single out, take control of, teleport, freeze, or reposition one player for a close-up. Ordinary hit-stop remains a separate, stronger contact beat.
4. **Preserve simulation semantics.** The fixed-tick game domain must still advance with the existing 60 Hz `TICK_SECONDS` and step order. Time dilation only changes *when* fixed ticks execute relative to real time, not their simulated delta or outcomes for the same tick-indexed input sequence. The evaluator never writes `CombatEvent`, collision, damage, RNG, death selection, score, or replay-hash inputs.
5. **No new art is required.** Reuse existing poses, death presenter, camera emphasis and sound where appropriate. HUD stays as designed; do not invent a slow-motion meter, subscriber count, view counter, QTE, replay cutscene or cinematic letterbox. The future audience/viewership scoring system must remain independent.
6. **Gameplay remains readable and playable.** Both players keep control; inputs must not be dropped or repeated because fewer fixed ticks run per real second. Threat tells, HUD, camera boundaries, and accessibility controls remain legible.
7. **One trigger per feat.** Deduplicate defeated targets, group defeats that resolve on one tick, do not stack slow effects or reset the duration on each extra death, and require a cooldown after the effect.
8. **Fail closed.** If the event source does not prove player ownership or a feat precondition, no cinematic trigger. Never fabricate player attribution from a cosmetic finish.

## 2. Evidence and eligibility (deterministic, headless)

Use a pure, Bevy-free `CinematicEvaluator` that **observes** the ordered event stream and stores a bounded rolling history (90 simulation ticks). It returns `Option<CinematicMoment>` for the presentation adapter. Replays with the same events must yield the same moments even if the presentation feature is switched off.

### Eligibility gate

Every moment must:

- Resolve from at least one actual `CombatEvent::EnemyDefeated` (once per living target; do not count extra corpses or synthetic stress/lab deaths).
- Be credited to a **known player** by the defeating action or the verified action chain. When the cause has a `player` field (for example `ShoreBlock { player }`), use that source; do not confuse `instigator` (which may identify an earlier hit) with the current prop operator. For direct melee, use its authoritative attacker; the current legacy Carl-only path may use its documented player-zero default, but **do not generalize that default to Donut or 2P**.
- Have a verified **primary skill feat** in the table below; bonus points alone can never qualify.
- Have no `HeroHurt` of the *credited hero* in the preceding `clean_window_ticks` (including this tick). An unrelated P2 hit does not disqualify P1.
- Not occur during defeat/restart, scripted camera captures, app pause, or other externally locked time flow.
- Pass the global cooldown, and not be part of a previously rewarded chain.

Process multiple events on the same tick as one batch, ordered deterministically by (tick, target ID), independent of iteration/renderer order. Use stable feature IDs and fixed integer scoring: no floating-point score thresholds.

### Primary feats and bonuses — initial v0 scoring

| Evidence ID | Required observable action | Score | Ready now? |
| --- | --- | ---: | --- |
| `CINE-PRECISION-EDGE` | A player-authored `ShoreBlockEdge` defeat after that player's verified shore break; the upright neck-edge check is what made it a *shear*, not an ordinary crush | **5** | **Depends on DEATH-D**, which is still specified but not implemented on `main` at this snapshot |
| `CINE-MIXED-CHAIN` | Same player, **two different living targets** defeated within 90 ticks, one by direct melee and one by that player's activated falling shore block; no player hurt in the interval | **5** | **Yes** with `Melee`, `ShoreBlock`, `ShoreBroken` and `EnemyDefeated` |
| `CINE-CONTACT-SETUP` | A previously player-launched *living* goblin made a recorded `GoblinContact`, and a linked target is defeated in the same causal window. The launch-to-player association must be proved by the hit event, never inferred from screen proximity | +2 | Conditionally; omit bonus if attribution is ambiguous |
| `CINE-EXTRA-KILL` | An additional distinct, credited enemy defeat within the same valid chain/window | +1 each, **max +2** | Yes |
| `CINE-COUNTER` (future) | A dedicated authoritative dodge/interrupt-to-counter evidence event with explicit player credit; cannot be inferred solely from a generic `GoblinInterrupted` | TBD | **Not implementable yet; do not synthesize it** |
| `CINE-DUO` (future) | Verified complementary actions by both human players in a causally linked kill | TBD | **Not implementable yet; do not pretend that a solo companion action is P2** |

**Thresholds:** ordinary cinematic at score **5–7**; legendary at score **8+**. Legendary is not a second execution: it is a slightly stronger variant of the **one** selected moment. Every qualifying score must include either `CINE-PRECISION-EDGE` or `CINE-MIXED-CHAIN`. Bonuses cannot activate slow motion on their own, even if an explosion kills five enemies. If several candidates qualify in the same batch, choose the highest score, then the lowest credited player ID and lowest target ID as deterministic tie-breakers. One candidate can upgrade a *pending* moment before it starts, but cannot retrigger/extend an active moment.

**Examples (binding expectations):**

- One jab, haymaker, push-kick or lethal blow with dramatic gibs: **no slow motion**.
- Three ordinary enemies killed simultaneously by an environmental blast: **no slow motion**; multiple corpses and catastrophic force do not prove exceptional execution.
- One player times CP-03 so the block shears an upright goblin's head (once DEATH-D exists): **ordinary cinematic**.
- A player's melee kill followed rapidly by their falling-block kill on a second target: **ordinary cinematic**.
- That mixed-cause chain also meaningfully includes a verified launch/contact setup and an additional defeat: **legendary** (5 + 2 + 1 = 8).
- The block falls without player attribution or a valid primary feat: **no slow motion**.
- A normal kill with a `goblin_decapitated` visual forcibly selected by a QA/lab override: **no slow motion**.
- P1 earns a feat while P2 gets hurt: **P1 still qualifies**; P2's unrelated events do not steal the credit.

### Event handling and future compatibility

- Take events **after** their authoritative tick has resolved and **before** the existing log-consumer cursor advances; inspect `CombatState::take_new_events` and the `print_combat_events` path to avoid double-consuming it. An independent read-only cursor or an explicit single fan-out to audio, HUD and cinema is acceptable.
- Build feat history from semantic events (`MoveStarted`, `Hit`, `GoblinContact`, `ShoreBroken`, `EnemyDefeated`, `HeroHurt`), but **do not require every event to have a new schema field**. Add a narrowly scoped player-credit bridge only if needed for correctness; a schema/hash migration is forbidden without its own explicit explanation and golden updates.
- Death recipe selection in `src/death.rs` and the `DeathPresenter` in `src/death_present.rs` are **outputs, not inputs** to this evaluator. Preset 1 vs 4 and disabled blood must produce the same eligibility decision.
- Keep the evaluator generic as new environment kills, species, player attacks, and viewership are added. Future feat categories require new authoritative evidence and tests, not hidden changes to the current scoring table.

## 3. Tunable cinematic presentation

The constants below are **initial playtest defaults**, kept together in a `CinematicTuning` value or equivalent, with a single documented place to adjust them. They are **not** gameplay balance and must not be serialized into authoritative replay state.

| Knob | Default | Meaning |
| --- | ---: | --- |
| `history_window_ticks` | 90 | 1.5 s of authored game time for a linked chain at 60 Hz |
| `clean_window_ticks` | 90 | No hurt to the credited hero during the feat window |
| `cooldown_ticks` | 900 | 15 s of normal simulation time before another cinematic may trigger |
| `ordinary_threshold` | 5 | Lowest eligible primary-feat score |
| `legendary_threshold` | 8 | Rarer tier |
| `ordinary_scale` | 0.35 | Virtual time multiplier for the world |
| `legendary_scale` | 0.25 | Virtual time multiplier for the world |
| `ordinary_duration_ms` | 480 | Real-time slow-motion window, **after** hit-stop |
| `legendary_duration_ms` | 650 | Real-time slow-motion window, **after** hit-stop |
| `blend_in_ms` | 35 | Short entrance, no hard speed discontinuity |
| `blend_out_ms` | 90 | Fast return to 1.0× |
| `camera_emphasis_px` | 2 | **Maximum** optional virtual-pixel emphasis; never forces another zoom/reframe |

Durations include their entrance and exit blends. When a candidate qualifies, **queue** the moment until its lethal contact's normal hit-stop has completed; do not add a second freeze or stack hit-stop values. If the death presenter no longer has an active finish when the pending moment would start, skip it rather than slowing empty scenery. The outcome must be a visible *defeat sequence*, not a slow-motion lull.

Implement as an explicit state machine: `Idle -> Pending -> Active -> Cooldown -> Idle`. The presentation adapter is the sole owner of the temporary world-scale override. Use the appropriate Bevy 0.20.1 virtual/fixed-time mechanism after confirming its current API, preserving constant **simulation tick delta**; use **unscaled real elapsed time** for cinematic duration. Recover the existing time scale (normally 1.0×) on completion, pause, retry, checkpoint respawn, app exit, and any exceptional path. Do not permanently alter the shared clock.

Frame pacing: no unbounded fixed-update catch-up, stutter-inducing CPU spikes, or duplicate input-edge consumption at the transition. Controller and keyboard press edges sampled in `Update` must remain buffered and consumed **once** in a fixed tick; verify the current `Controls`/`PadSlots` buffering logic before adjusting time flow. A pause stops cinematic real-time progression until resumed (or cancels cleanly); choose and test **cancel on pause**, restoring 1.0× before showing a pause menu.

**Camera and audio:** prefer no camera zoom at v0. A tiny optional pixel-snapped emphasis may reuse the existing camera kick, bounded by view geometry and **never** forcing either co-op hero outside the approved shared-camera safe area. Do not move the authoritative `ScrollCamera`. A single existing impact accent plus a short optional ambience/music duck is enough; do not pitch-shift the full soundtrack, repeat the lethal sound on resume, or add assets/dependencies just for this mechanic. The effect remains satisfying if camera emphasis and audio accent are disabled.

**Comfort controls:** expose `cinematic_finishes: on/off` and `reduced_motion` as presentation settings (a CLI/debug flag is acceptable until the settings menu exists). Both remove time dilation and camera emphasis, not normal hit confirmation/death VFX or combat outcome. They must not affect evaluator eligibility or replay hashes. For automated/headless fixtures, default presentation **off** unless explicitly running a CINE-01 visual probe; the evaluator's pure tests still run.

## 4. Implementation handoff (one local agent, one implementation PR)

**Scope:** implement *only* CINE-01 on top of the current merged code. No viewership mechanics, subscriber system, HUD replacement, new enemy attacks, QTE, cinematic camera subsystem, gameplay combo meter, expensive new art, or refactor of general time management. Do not implement DEATH-D inside this work order: integrate its event when it is merged, and keep the feature operational using `CINE-MIXED-CHAIN` meanwhile.

Suggested footprint (adjust against the actual branch before editing):

1. `src/cinematic.rs` — pure bounded evidence collector, eligible-feat scoring, deterministic tie-breaks and tuneable `CinematicTuning`; exported from `src/lib.rs`.
2. `src/bin/floor01_scene.rs` — feed observed combat events once per resolved tick, run `Pending/Active/Cooldown` and apply/remove the temporary Bevy world time multiplier; ensure input buffering, pause/respawn and old replay modes work.
3. `src/audio_adapter.rs` or the existing presentational audio pathway **only if** an existing accent can be reused without schema changes. Otherwise omit this from v0.
4. Unit tests in the pure module; one headless fixture or probe with negative, positive and multi-kill cases; one real Bevy rendered capture with feature on/off. Document precise commands and hashes in an evidence README.
5. Update the backlog status, feature documentation and any user-facing setting help; report changed tests, measured frame pacing and playtest uncertainties in the implementation PR.

**Agent command:** read `AGENTS.md`, this document, the listed system modules and the latest open PRs first. Present a five-step plan with files and any conflicts, then implement. Never claim this documentation PR already delivered code or visuals. Independent top-tier review + green CI precede automated merge; **owner game-feel acceptance is after a real solo and two-controller playtest** per the repo's policy.

## 5. Acceptance tests — required before claiming implementation

| Test ID | Fixture / action | Required result |
| --- | --- | --- |
| CINE-T01 | Ordinary melee kill, any force / gore preset, including a forced gory visual | No candidate; virtual time remains at 1.0× |
| CINE-T02 | Unattributed/mocked catastrophic explosion or three routine same-source kills | No candidate even with 3+ corpses |
| CINE-T03 | Credited melee + shore kill on distinct enemies within 90 ticks; no credited-hero hurt | One ordinary moment, score 5; **not** one per kill |
| CINE-T04 | Same as T03 outside the window, or after hurt to the credited hero | No cinematic |
| CINE-T05 | DEATH-D credited edge shear with a verified shore break (when available) | One ordinary moment; normal crushing/blunt decap override never qualifies |
| CINE-T06 | Mixed chain + verified live-goblin contact setup + third credited kill | One legendary moment, score 8; no stacking |
| CINE-T07 | Repeat an eligible chain inside 900 simulation ticks; then after cooldown | First only during cooldown, next eligible after cooldown |
| CINE-T08 | Same tick multiple defeats, repeated event, reordered insertion of otherwise equivalent evidence | Deterministic single choice and tie-break; no repeated credit |
| CINE-T09 | Same tick-indexed inputs with cinematic on/off and gore presets 1/4 | **Identical authoritative state, ordered combat events and replay hash** |
| CINE-T10 | Local 2P: P1 qualifies while P2 moves/attacks/is hit; solo swap and controller ownership | One shared slowdown, P2 inputs preserved; P2 hurt does not veto P1 |
| CINE-T11 | Pause, restart, checkpoint respawn, scene exit, hit-stop overlap and disable/reduced motion | No stuck slow mode, no doubled freeze, no dropped/duplicated presses |
| CINE-T12 | Real Bevy capture at 640×360 and 640×400; 1× and 3× integer scale | Legible danger, no camera jitter/clipping/HUD shift, noticeable moment, clean return to full speed |
| CINE-T13 | Worst credible simultaneous defeat stress and normal uninterrupted play | Bounded evaluator memory, no unbounded allocation or fixed-step catch-up, frame-time evidence reported |

T05 may be marked **blocked on DEATH-D** while that work is unmerged, but the fixture and expectation must be prepared. If an unimplemented rule cannot be exercised, report it as unimplemented, not passed. If the runtime's existing fixtures cannot produce T03 without new layout work, add the **smallest deterministic QA fixture**—never change the main route solely for this feature.

**Manual owner playtest questions:** Does an ordinary kill still feel satisfying? Is the effect rare enough after ten minutes of good play? Do truly clever environmental chains get recognized? Does either player feel robbed of control in 2P? Is 0.48/0.65 s too long after hit-stop? Are there FPS drops or lost button taps? Tune the named knobs and explicit feat rules from evidence, in follow-up PRs; do not hard-code unreviewed magic numbers across unrelated systems.

## 6. Explicit non-goals and status

This document **approves the design intent and adopts a concrete v0 for an agent to build**. It does **not** claim cinematic slow motion is running, that the numbers feel right, or that DEATH-D, real 2P testing, or later counter/duo evidence is merged. The planned audience/viewership HUD system will later be free to observe these moments, but **cannot cause them**. The game should be fun without a scoreboard, and ordinary combat must not be slowed down to manufacture spectacle.

## 7. Implementation status

**Phase 1 (evaluator): in progress, unit-tested, not wired.** `src/cine.rs` is Bevy-free and touches no simulation file; nothing runs in the game yet and no game-feel claim exists. Phase 2 (the adapter) waits for the boss code that edits `combat.rs`, `combat_scene.rs`, `corridor.rs` and `floor01_scene.rs`. The module is `src/cine.rs` (the work order's suggested `src/cinematic.rs` is not used).

### What exists

| Piece | Role |
| --- | --- |
| `CineTuning` (`CineTuning::DEFAULT`) | The **one** tuning table: every spec number of sections 2 and 3, plus three marked assumptions (`max_history_entries` 64, `upgrade_window_ticks` 8, `edge_buffer_cap` 1) and `contact_setup_enabled` |
| `CineEvent`, `CineCause` | Plain-data mirror of the combat facts the evaluator reads; `CineEvent::from_combat` is the whole mapping for events `CombatEvent` already carries |
| `CinematicEvaluator::observe_tick(tick, events, locked)` | One batch per resolved tick; fail-closed credit, rolling bounded history, scoring, tie-breaks, cooldown, upgrade; returns `Option<CinematicMoment>`; `set_scripted_scene`, `reset` |
| `CineTuning::request_for(&moment)` -> `SlowMoRequest` | Tier, scale, duration, blends, camera emphasis, cooldown. Presentation only |
| `SlowMoPlanner` (`request`, `step(FrameContext) -> FramePlan`) | `Idle -> Pending -> Active -> Idle`, real-time driven, non-stacking, pause/lock/setting cancellation. The spec's `Cooldown` state lives in the evaluator (simulation ticks) |
| `EdgeLatch` | Press-edge buffer with the same semantics as the current `Controls`/`PadSlots` latch |
| `PresentationSettings` | `cinematic_finishes` and a **separate** cinematic `reduced_motion`; env `FLOORBREAK_CINEMATIC_FINISHES`, `FLOORBREAK_CINEMATIC_REDUCED_MOTION`, flags `--cinematic-finishes[=on|off]`, `--cinematic-reduced-motion[=on|off]` |

### Decisions applied from the independent spec review (F1 to F10)

- **Credit is the hero index** (Carl 0, Donut 1), carried by the event; "same player" means same hero, so a Donut kill and a block broken as Carl never chain, even under a solo swap. The owning slot at the event tick is carried next to it (`CineEvent::Defeated.slot`, `CinematicMoment.credited_slot`) for naming only. The clean window checks `HeroHurt` of the credited hero only.
- **Block credit fails closed:** a `ShoreBlock`/`ShoreBlockEdge` defeat counts only if a `ShoreBroken` by the very player the cause names was seen in the same attempt (held outside the 90-tick window, one per breaking hero). The sim's own "no breaker means P1" default is therefore never credited.
- **Restart** clears history, deduplication, the shore breaker and the cooldown (the attempt-local tick restarts at 1 and target ids are reused). **Respawn** clears history and the lock but keeps the cooldown.
- **Legendary is unreachable by default.** "A linked target" is undefined, and `CINE-CONTACT-SETUP` is the only way to 8 (5 + capped 2 = 7). `contact_setup_enabled` is `false`; the linkage implemented behind it is the reviewer's proposal (a launched goblin's contact `source` or `target` defeated by the credited hero within the history window of the contact), tested with the flag on.
- **Separate cinematic settings**, not the HUD reduced-motion flag. Reduced motion and `cinematic_finishes: off` suppress the request entirely (no downgrade); the evaluator ignores both.
- **Scripted scenes:** `set_scripted_scene(true)` records but never emits (DEMO-01 B3 would otherwise qualify on every run). The adapter sets it for the demo route.
- **Pause** means the D6 disconnect pause and `Frozen` captures (there is no pause menu): the planner cancels to 1.0x and never resumes. **Hit-stop** during `Active` is accepted, dilated with the world, and does not cancel.
- Boss defeats: the boss arena has one target, so no chain can form; boss design section 5 ("no slow motion") stays authoritative, and the evaluator has no boss special case.

### Test coverage of CINE-T01 to T13 (`cargo test --lib cine::`)

| Test | Phase 1 (pure) | Needs the adapter phase |
| --- | --- | --- |
| T01, T02 | covered: no primary feat, force never reaches the evaluator, unattributed/other causes | forced visual flag and preset 4 run (windowed), "virtual time stays 1.0x" on a real clock |
| T03, T04 | covered: chain, window boundary, clean window, both kill orders | positive fixture on B3/pileup (windowed) |
| T05 | evaluator side covered with a prepared `ShoreBlockEdge` cause | blocked on DEATH-D: no such cause exists in `CombatEvent` |
| T06 | covered **only with** `contact_setup_enabled`; default proves legendary unreachable | needs the F4 definition and the `LaunchCredited` bridge |
| T07 | covered: cooldown, boundaries, restart/respawn | continuous run across a real restart |
| T08 | covered: same-tick batch, tie-break, rotations/reversal, repeated events | none |
| T09 | evaluator parity covered by construction (no force/gore input) | windowed run on/off and hash diff |
| T10 | covered: one shared moment, P2 hurt does not veto P1, independent edge latches | manual two-controller check; no scripted 2P fixture exists |
| T11 | planner cancellation (pause, lock, settings, skip with no finish), hit-stop wait, edge latch | real `Time<Virtual>` restore, scene exit, restart in the Bevy app |
| T12 | none | real Bevy capture at both resolutions and scales |
| T13 | evaluator memory bound (stress of 40 defeats per tick), no catch-up burst in the simulated loop | measured frame times on the real runtime |

### Adapter phase TODO (after the boss code merges; not done here)

1. Fan events out once in `print_combat_events` (a `&mut` cinematic sink next to audio), covering `fixed_step` (twice), `watchdog` and `capture_sequence`; do not add a second cursor inside `combat_scene.rs` (the 512-event drain adjusts the single cursor). Map with `CineEvent::from_combat`, attach `with_slot` from `PartyState`, and call `observe_tick` once per resolved tick with the batch.
2. Add the missing evidence: a `LaunchCredited` event derived from `Hit { PushKick, instance == GoblinContact.launch, target == source }` (only once F4 is defined and `contact_setup_enabled` is switched on), `ShoreBlockEdge` when DEATH-D merges, and the boss design's `DropHazard { player, rack }` credit in `cine_cause`.
3. Own the clock: apply `FramePlan.relative_speed` to `Time<Virtual>` (confirm the Bevy 0.20.1 API), keep `Time<Fixed>` at 60 Hz, and feed `FrameContext` each frame: real delta, the lethal hit's hit-stop ticks left, whether the death presenter has an active finish, `paused` from D6, `locked` for `Frozen`, defeat, restart, respawn, exit and `--capture-tick`. Restore 1.0x on every cancel and on scene exit.
4. Mark DEMO-01 as a scripted scene (`set_scripted_scene(true)`); keep headless fixtures at `PresentationSettings::OFF` by default.
5. Replace the three `Controls` latches that matter with `EdgeLatch` only if the owner picks a cap above 1; the current latch already keeps a press across a frame with zero ticks.
6. Apply `camera_emphasis_px` through the existing camera kick only; no change to `ScrollCamera`. Note that `audio_adapter` reads virtual time, so its fades stretch during slow motion.
7. Evidence: windowed on/off run with identical printed hash and event log (T09), Bevy captures (T12), frame-time numbers (T13), and a manual solo and two-controller playtest.

### Spec ambiguities recorded while implementing

- **Cooldown start and unit.** Counted in simulation ticks from the tick a moment is emitted (not from its end); the spec does not say.
- **Upgrade window.** "One candidate can upgrade a pending moment" has no number; the 8-tick `upgrade_window_ticks` is an assumption.
- **Boundaries.** "Within 90 ticks" is read as at most 90 ticks apart (inclusive).
- **Double-press inside one slowed tick.** At 0.25x a tick lasts about 67 ms of real time, so two presses closer than that collapse into one with the current latch (counted in `EdgeLatch::coalesced`); `edge_buffer_cap` 2 would keep both. This is a game-feel decision.
- **Verified break and cause player.** A `ShoreBlock` already names its player; requiring a matching `ShoreBroken` as well is the conservative reading.

