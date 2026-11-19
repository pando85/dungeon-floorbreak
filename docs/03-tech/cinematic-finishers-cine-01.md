# CINE-01 — Skill-gated cinematic slow motion for exceptional defeats

**Status:** owner-approved *creative intent* (2026-11-19); implemented in the CINE-01 PR (see section 8), **game feel pending**. This document is the implementation work order for a local coding agent. The numerical defaults below are **adopted starting hypotheses**, not owner-approved game feel.

**Goal:** In THE CRAWL, an unusually well-executed defeat briefly makes the action feel cinematic. The player earns that moment by timing, setting up an environmental kill, or constructing a meaningful chain—not by killing an ordinary enemy or selecting a violent death recipe. The effect must be **rare**, unmistakable, and over before it interrupts the pace.

Read first: `AGENTS.md`; [combat feel](../01-design/combat-feel-hybrid-physics.md); [universal defeat language](../01-design/universal-monster-death-language-v0.md); [death architecture](monster-death-system-proposal.md); [DEATH-D](death-d-finishes-spec.md); [party design](../01-design/party-2p-open-questions.md); [camera/accessibility](input-camera.md); and [work order protocol](../07-agents/workorders.md). Respect the [long-term character horizon](../01-design/character-progression-horizon.md): this is a presentation reward for existing verbs, **not** a new move, unlock, resource, or progression ability.

## 1. Non-negotiable behavior

1. **Exceptional execution is required.** A death is necessary but not sufficient. Neither `ForceTier::Catastrophic`, gore preset 4, a dismemberment recipe, an explosion, high damage, nor a boss death automatically triggers slow motion. In v0 a boss defeat never triggers CINE, per the boss design's "no slow motion or cinematic pause" rule (see [Floor 1 boss design](../02-art/floor-01-boss-design.md) section 5); any change is an owner decision.
2. **Causality over chance.** Use observed, authoritative combat events and verified player attribution. No random chance, visual-debris collisions, VFX counts, estimated audience/viewership, or opaque “coolness” heuristics.
3. **One shared world, one shared camera.** In solo and local 2P, both heroes, all enemies and hazards undergo the same brief time dilation. Do not single out, take control of, teleport, freeze, or reposition one player for a close-up. Ordinary hit-stop remains a separate, stronger contact beat.
4. **Preserve simulation semantics.** The fixed-tick game domain must still advance with the existing 60 Hz `TICK_SECONDS` and step order. Time dilation only changes *when* fixed ticks execute relative to real time, not their simulated delta or outcomes for the same tick-indexed input sequence. The evaluator never writes `CombatEvent`, collision, damage, RNG, death selection, score, or replay-hash inputs.
5. **No new art is required.** Reuse existing poses, death presenter, camera emphasis and sound where appropriate. HUD stays as designed; do not invent a slow-motion meter, subscriber count, view counter, QTE, replay cutscene or cinematic letterbox. The future audience/viewership scoring system must remain independent.
6. **Gameplay remains readable and playable.** Both players keep control; no press edge is lost and none is repeated because fewer fixed ticks run per real second; presses within one fixed tick count once (the existing latch). Threat tells, HUD, camera boundaries, and accessibility controls remain legible.
7. **One trigger per feat.** Deduplicate defeated targets, group defeats that resolve on one tick, do not stack slow effects or reset the duration on each extra death, and require a cooldown after the effect.
8. **Fail closed.** If the event source does not prove player ownership or a feat precondition, no cinematic trigger. Never fabricate player attribution from a cosmetic finish.

## 2. Evidence and eligibility (deterministic, headless)

Use a pure, Bevy-free `CinematicEvaluator` that **observes** the ordered event stream and stores a bounded rolling history (90 simulation ticks). It returns `Option<CinematicMoment>` for the presentation adapter. Replays with the same events must yield the same moments even if the presentation feature is switched off.

### Eligibility gate

Every moment must:

- Resolve from at least one actual `CombatEvent::EnemyDefeated` (once per living target; do not count extra corpses or synthetic stress/lab deaths).
- Be credited to a **known hero** by the defeating action or the verified action chain (see "Credit identity" below). When the cause has a `player` field (for example `ShoreBlock { player }`), use that source; do not confuse `instigator` (which may identify an earlier hit) with the current prop operator. For direct melee, use its authoritative attacker; the current legacy Carl-only path may use its documented player-zero default, but **do not generalize that default to Donut or 2P**.
- Have a verified **primary skill feat** in the table below; bonus points alone can never qualify.
- Have no `HeroHurt { hero }` with `hero == credited hero` in the preceding `clean_window_ticks` (including this tick). A hurt to a different hero (for example Donut while Carl is credited) does not disqualify the credited hero.
- Not occur during defeat/restart, scripted camera captures, a `Frozen` capture, the D6 disconnect pause, or other externally locked time flow.
- Pass the global cooldown, and not be part of a previously rewarded chain.
- For any `ShoreBlock` or `ShoreBlockEdge` credit: have a `ShoreBroken { player }` observed in the **same attempt** with the same player as the defeat cause. `step_shore` credits `ShoreBlock { player: shore.breaker.unwrap_or(PLAYER_ONE) }` when a block has no breaker, so a block defeat without a matching `ShoreBroken` is treated as unattributed and gives no credit. Track the last `ShoreBroken` per attempt **outside** the rolling window, because the block's fall time can exceed it.

**Credit identity (v0).** Credit is the **hero index** carried by the event (0 is Carl, 1 is Donut), which matches main (`Attacker.player`, `ShoreBroken.player`) and the Q10 pounce credit. "Same player" in this document means the same hero. The clean-window veto checks `HeroHurt { hero == credited hero }`. Under solo swap this means no chain across heroes: for example, a pounce kill as Donut (credit 1) followed by a block Carl broke (credit 0) is two separate credits, not one chain. Mapping human controller slots to heroes is left to `CINE-DUO`. Main (`goblin.rs` DEATH-B comment, `Attacker.player`) calls this value a player slot while Q10 calls it a hero index; the owner may want to unify the naming.

**Attempt boundaries and clocks.** Ticks are attempt-local: `Restart` resets the tick counter and is not hashed, and target IDs are reused after a restart. The evaluator therefore clears its history, per-target deduplication and last-`ShoreBroken` records on `Restart` and on `Respawn` (DEMO-01), and the adapter cancels any `Pending` or `Active` moment at the same time. The cooldown is measured on a monotonic counter (`CombatState::elapsed` or the evaluator's own count of observed ticks), not on attempt-local ticks, and it **survives** `Restart` and `Respawn`.

Match the "player-activated drop hazard" cause class through one helper (for example `DefeatCause::drop_credit() -> Option<u32>`) rather than hard-coding `ShoreBlock`. The merged boss design adds `DefeatCause::DropHazard { player, rack }` as a **sibling variant of `ShoreBlock` with no hash migration** (CP-03 hashes stay byte-identical), and the helper must cover `ShoreBlock`, `ShoreBlockEdge` and `DropHazard`.

Process multiple events on the same tick as one batch, ordered deterministically by (tick, target ID), independent of iteration/renderer order. Use stable feature IDs and fixed integer scoring: no floating-point score thresholds.

### Primary feats and bonuses — initial v0 scoring

| Evidence ID | Required observable action | Score | Ready now? |
| --- | --- | ---: | --- |
| `CINE-PRECISION-EDGE` | A player-authored `ShoreBlockEdge` defeat after that player's verified shore break; the upright neck-edge check is what made it a *shear*, not an ordinary crush | **5** | **Yes**: DEATH-D is merged (#108) and `ShoreBlockEdge { player }` exists on main; fixtures `cp03_block_edge_decap` and DEMO-01 B3 |
| `CINE-MIXED-CHAIN` | Same hero, **two different living targets** defeated within 90 ticks, one by direct melee and one by that hero's activated falling shore block (any order); the credited hero is not hurt in the interval | **5** | **Yes** with `Melee`, `ShoreBlock`, `ShoreBroken` and `EnemyDefeated` |
| `CINE-CONTACT-SETUP` | A previously player-launched *living* goblin made a recorded `GoblinContact`, and a **linked** target is defeated by the credited hero. *Linked* means the contact's `source` or `target` is defeated by the credited hero within `history_window_ticks` of the contact. The launch-to-player association is proved by `GoblinContact.launch` equalling the `instance` of a `Hit { move_id: PushKick, target == source }` in history, never inferred from screen proximity | +2 | Conditionally; omit bonus if attribution is ambiguous |
| `CINE-EXTRA-KILL` | An additional distinct, credited enemy defeat within the same valid chain/window | +1 each, **max +2** | Yes |
| `CINE-COUNTER` (future) | A dedicated authoritative dodge/interrupt-to-counter evidence event with explicit player credit; cannot be inferred solely from a generic `GoblinInterrupted` | TBD | **Not implementable yet; do not synthesize it** |
| `CINE-DUO` (future) | Verified complementary actions by both human players in a causally linked kill | TBD | **Not implementable yet; do not pretend that a solo companion action is P2** |

**Thresholds:** ordinary cinematic at score **5–7**; legendary at score **8+**. Legendary is not a second execution: it is a slightly stronger variant of the **one** selected moment. Without `CINE-CONTACT-SETUP`, the maximum is 5 + 2 extra kills = 7, so **legendary is unreachable unless that linked contact proof exists**. Every qualifying score must include either `CINE-PRECISION-EDGE` or `CINE-MIXED-CHAIN`. **Primaries do not stack**: a candidate scores exactly one primary (5) plus bonuses, even if it satisfies both (for example a melee kill, then a landing that shears one goblin and crushes another). `ShoreBlockEdge` counts as a "falling shore block" for `CINE-MIXED-CHAIN` through the drop-credit helper, so legendary stays unreachable without `CINE-CONTACT-SETUP`. Bonuses cannot activate slow motion on their own, even if an explosion kills five enemies. If several candidates qualify in the same batch, choose the highest score, then the lowest credited hero index and lowest target ID as deterministic tie-breakers. One candidate can upgrade a *pending* moment before it starts, but cannot retrigger/extend an active moment. An upgrade is accepted only while the moment is `Pending` **and** within 8 ticks of its first qualifying tick; `Pending` is never extended to wait for an upgrade.

**Examples (binding expectations):**

- One jab, haymaker, push-kick or lethal blow with dramatic gibs: **no slow motion**.
- Three ordinary enemies killed simultaneously by an environmental blast: **no slow motion**; multiple corpses and catastrophic force do not prove exceptional execution.
- One player times CP-03 so the block shears an upright goblin's head (`ShoreBlockEdge`): **ordinary cinematic**.
- A player's melee kill and their falling-block kill on a second target within the window, **in either order**: **ordinary cinematic**. The 90-tick window is measured between the two defeats. `cp03_goblin_pileup` (melee at tick 83, block at tick 139, score 7) is the mixed-chain fixture.
- The authored DEMO-01 B3 beat on merged DEATH-D (`shore_broken` at tick 123, then at tick 159 target 4 `shore_block` and target 5 `shore_block_edge`, no hurt; checked by running the B3 headless probe on main): the first moment fires at **tick 159** as `CINE-PRECISION-EDGE` (5 + 1 extra kill = 6, ordinary). The haymaker kill at tick 243 (target 6) falls inside the 900-tick cooldown and gives nothing. So a competent demo run is expected to earn its **first cinematic at B3**. This is accepted as intended; rarity is kept by the cooldown and the clean-window requirement and is a playtest tuning question.
- That mixed-cause chain also meaningfully includes a verified launch/contact setup and an additional defeat: **legendary** (5 + 2 + 1 = 8).
- The block falls without player attribution or a valid primary feat: **no slow motion**.
- A normal kill with a `goblin_decapitated` visual forcibly selected by a QA/lab override: **no slow motion**.
- P1 earns a feat while P2 gets hurt: **P1 still qualifies**; P2's unrelated events do not steal the credit.

### Event handling and future compatibility

- Take events **after** their authoritative tick has resolved and **before** the existing log-consumer cursor advances; inspect `CombatState::take_new_events` and the `print_combat_events` path to avoid double-consuming it. Prefer a single fan-out inside `print_combat_events` (a `&mut` cinematic sink passed alongside audio), because `push_event` drains at `EVENT_LOG_CAP` and adjusts the single cursor, so a second cursor would need the same adjustment inside `combat_scene.rs`. Events are consumed at four sites (`fixed_step` twice, `watchdog`, `capture_sequence`): cover all of them or document why the watchdog and capture paths skip it.
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

Durations include their entrance and exit blends. When a candidate qualifies, **queue** the moment until its lethal contact's normal hit-stop has completed; do not add a second freeze or stack hit-stop values. `Pending` counts the lethal hit's hit-stop in **simulation ticks**. Hit-stop is also in ticks, so a hit-stop that starts during `Active` is accepted as dilated by 1/scale (8 ticks at 0.35 is about 381 ms real); it is not a cancel condition. T11 pins this choice. If the death presenter no longer has an active finish when the pending moment would start, skip it rather than slowing empty scenery. The outcome must be a visible *defeat sequence*, not a slow-motion lull.

The presentation adapter runs an explicit state machine `Idle -> Pending -> Active -> Idle` and is the sole owner of the temporary world-scale override. The **cooldown lives in the evaluator**: it starts when the moment is emitted, regardless of whether the adapter skips or cancels the moment (CINE off, no active finish, D6 pause), which keeps it deterministic. Use the appropriate Bevy 0.20.1 virtual/fixed-time mechanism after confirming its current API, preserving constant **simulation tick delta**; use **unscaled real elapsed time** for cinematic duration. Recover the existing time scale (normally 1.0×) on completion, the D6 disconnect pause, a `Frozen` capture, retry, checkpoint respawn, app exit, and any exceptional path. Do not permanently alter the shared clock.

Frame pacing: no unbounded fixed-update catch-up, stutter-inducing CPU spikes, or duplicate input-edge consumption at the transition. Controller and keyboard press edges sampled in `Update` must remain buffered and consumed **once** in a fixed tick; verify the current `Controls`/`PadSlots` buffering logic before adjusting time flow. No pause menu exists on main. The only pauses are the D6 disconnect pause (`pads.is_paused()`: `fixed_step` returns early while `Time<Virtual>` keeps running, so a real-time duration would expire unseen) and `Frozen` captures. **Cancel on D6 pause or `Frozen`**, restoring 1.0× at once.

**Camera and audio:** prefer no camera zoom at v0. A tiny optional pixel-snapped emphasis goes through the scene camera's existing kick input (`src/scene_camera.rs`, the presentation-only auto camera added in #112) and **never** drives its zoom or pan, and is bounded by view geometry and never forces either co-op hero outside the approved shared-camera safe area. Do not move the authoritative `ScrollCamera`. The scene camera's tick-paced easing slowing down during `Active` is expected. A single existing impact accent plus a short optional ambience/music duck is enough; do not pitch-shift the full soundtrack, repeat the lethal sound on resume, or add assets/dependencies just for this mechanic. The effect remains satisfying if camera emphasis and audio accent are disabled.

**Comfort controls:** expose `cinematic_finishes: on/off` as a presentation setting through `--cinematic-finishes[=on|off]` and `FLOORBREAK_CINEMATIC_FINISHES`, following the `HudSettings` env-then-flag layering (a CLI/debug flag is acceptable until the settings menu exists). Do **not** reuse `--hud-reduced-motion`: `HudSettings.reduced_motion` is documented as HUD-only and world motion is deferred to HUD-DEBT-08. World reduced motion is routed through HUD-DEBT-08 (a world-motion/camera-shake setting), which must also disable CINE. Turning CINE off removes time dilation and camera emphasis, not normal hit confirmation/death VFX or combat outcome. These settings must not affect evaluator eligibility or replay hashes. For automated/headless fixtures, default presentation **off** unless explicitly running a CINE-01 visual probe; the evaluator's pure tests still run.

## 4. Implementation handoff (one local agent, one implementation PR)

**Scope:** implement *only* CINE-01 on top of the current merged code. No viewership mechanics, subscriber system, HUD replacement, new enemy attacks, QTE, cinematic camera subsystem, gameplay combo meter, expensive new art, or refactor of general time management. Do not implement DEATH-D inside this work order: integrate its event when it is merged, and keep the feature operational using `CINE-MIXED-CHAIN` meanwhile.

Suggested footprint (adjust against the actual branch before editing):

1. `src/cine.rs` — pure bounded evidence collector, eligible-feat scoring, deterministic tie-breaks and tuneable `CinematicTuning`; exported from `src/lib.rs`.
2. `src/bin/floor01_scene.rs` — feed observed combat events once per resolved tick, run `Idle/Pending/Active` and apply/remove the temporary Bevy world time multiplier; ensure input buffering, pause/respawn and old replay modes work.
3. `src/audio_adapter.rs` or the existing presentational audio pathway **only if** an existing accent can be reused without schema changes. Otherwise omit this from v0.
4. Unit tests in the pure module; one headless fixture or probe with negative, positive and multi-kill cases; one real Bevy rendered capture with feature on/off. Document precise commands and hashes in an evidence README.
5. Update the backlog status, feature documentation and any user-facing setting help; report changed tests, measured frame pacing and playtest uncertainties in the implementation PR.

**Agent command:** read `AGENTS.md`, this document, the listed system modules and the latest open PRs first. Present a five-step plan with files and any conflicts, then implement. Never claim this documentation PR already delivered code or visuals. Independent top-tier review + green CI precede automated merge; **owner game-feel acceptance is after a real solo and two-controller playtest** per the repo's policy.

## 5. Acceptance tests — required before claiming implementation

| Test ID | Fixture / action | Required result |
| --- | --- | --- |
| CINE-T01 | Ordinary melee kill, any force / gore preset (use preset 4 for the gory case; do not use `--death-lab-finish`, which DEATH-D removes) | No candidate; virtual time remains at 1.0× |
| CINE-T02 | Three routine same-source kills, a block-only multi-kill with no matching `ShoreBroken`, or the `qa_blast_breakup` `Explosion` defeats (an explosion is never a primary feat) | No candidate even with 3+ corpses |
| CINE-T03 | Credited melee + shore kill on distinct enemies within 90 ticks, either order; no credited-hero hurt. Positive fixture: `cp03_goblin_pileup` (melee at 83, block at 139; no new layout needed) | One ordinary moment (score 5 plus bonuses, 7 for the pileup); **not** one per kill |
| CINE-T04 | Same as T03 outside the window, after hurt to the credited hero, or with a `ShoreBlock` defeat whose `ShoreBroken` was not observed in the same attempt (breaker defaulted to P1) | No cinematic |
| CINE-T05 | DEATH-D credited edge shear with a verified shore break: `cp03_block_edge_decap`, and DEMO-01 B3 (`shore_broken` at 123, `shore_block_edge` at 159) | One ordinary moment (B3: `CINE-PRECISION-EDGE`, score 6, tick 159); normal crushing/blunt decap override never qualifies |
| CINE-T06 | Mixed chain + verified live-goblin contact setup + third credited kill | One legendary moment, score 8; no stacking |
| CINE-T07 | Repeat an eligible chain inside 900 simulation ticks; then after cooldown (B3: the tick 243 haymaker falls inside the cooldown of the tick 159 moment); then `Restart` and `Respawn` (history and per-target dedup cleared, cooldown on the monotonic counter) | First only during cooldown, next eligible after cooldown; no stale or underflowed history after a restart |
| CINE-T08 | Same tick multiple defeats, repeated event, reordered insertion of otherwise equivalent evidence | Deterministic single choice and tie-break; no repeated credit. A candidate satisfying both primaries scores 5 plus bonuses once (primaries do not stack) and is never legendary without `CINE-CONTACT-SETUP` |
| CINE-T09 | Pure-evaluator parity test, plus the **windowed** binary on a qualifying fixture (B3 or the pileup) with `--exit-after-ticks`, CINE on/off and gore presets 1/4; diff the printed hash and event log | **Identical authoritative state, ordered combat events and replay hash** |
| CINE-T10 | Pure evaluator test (Carl credited, Donut `HeroHurt`; solo-swap credit split across heroes) plus the existing `PadSlots` latch test. The live two-controller check is **manual** (fixtures force `live_party = false` and Donut has no attacks on main) | One shared slowdown, P2 inputs preserved; Donut hurt does not veto Carl |
| CINE-T11 | D6 disconnect pause, `Frozen` capture, restart, checkpoint respawn, scene exit, hit-stop overlap (Pending counted in ticks; hit-stop during Active accepted as dilated) and CINE off | No stuck slow mode, no doubled freeze; no press edge lost or repeated (presses within one fixed tick count once, via the existing latch) |
| CINE-T12 | Real Bevy capture at 640×360 and 640×400; 1× and 3× integer scale | Legible danger, no camera jitter/clipping/HUD shift, noticeable moment, clean return to full speed |
| CINE-T13 | Worst credible simultaneous defeat stress and normal uninterrupted play | Bounded evaluator memory, no unbounded allocation or fixed-step catch-up, frame-time evidence reported |

If an unimplemented rule cannot be exercised, report it as unimplemented, not passed. If the runtime's existing fixtures cannot produce T03 without new layout work, add the **smallest deterministic QA fixture**—never change the main route solely for this feature.

**Manual owner playtest questions:** Does an ordinary kill still feel satisfying? Is the effect rare enough after ten minutes of good play? Do truly clever environmental chains get recognized? Does either player feel robbed of control in 2P? Is 0.48/0.65 s too long after hit-stop? Are there FPS drops or lost button taps? Tune the named knobs and explicit feat rules from evidence, in follow-up PRs; do not hard-code unreviewed magic numbers across unrelated systems.

## 6. Explicit non-goals and status

This document **approves the design intent and adopts a concrete v0 for an agent to build**. It does **not** claim cinematic slow motion is running, that the numbers feel right, or that real 2P testing or later counter/duo evidence is merged. The planned audience/viewership HUD system will later be free to observe these moments, but **cannot cause them**. The game should be fun without a scoreboard, and ordinary combat must not be slowed down to manufacture spectacle.

## 7. Adopted review decisions (2026-11-19)

Applied under the owner's design-defaults directive (AGENTS.md, 2026-11-19) from the independent review of PR #117. The owner's creative intent above is unchanged. The owner may change any row in a follow-up PR.

| Item | Choice | Reason | Owner may change |
| --- | --- | --- | --- |
| F1 credit identity | v0 credit is the hero index carried by the event (0 Carl, 1 Donut); no chain across heroes under solo swap; slot mapping left to `CINE-DUO` | Matches main and Q10; reviewer's stated fix | Yes |
| F2 shore credit | `ShoreBlock`/`ShoreBlockEdge` credit requires a same-attempt, same-player `ShoreBroken` | Closes the fail-closed hole from the P1 default breaker | Yes |
| F3 attempt boundaries | Clear history and dedup on `Restart`/`Respawn`; cooldown on a monotonic counter that survives them | Ticks are attempt-local and target IDs are reused | Yes |
| F4 linked target | Linked means the contact's `source` or `target` defeated by the credited hero within the history window, proved by the PushKick `Hit` instance; otherwise legendary is unreachable | Reviewer's stated fix | Yes |
| F5 settings | New `--cinematic-finishes[=on\|off]` / `FLOORBREAK_CINEMATIC_FINISHES`; no reuse of `--hud-reduced-motion`; world reduced motion via HUD-DEBT-08, which also disables CINE | Reviewer's recommended option; keeps the HUD-only flag scope | Yes |
| F6 B3 / pileup | Accept: the authored DEMO-01 B3 beat pays out as `CINE-PRECISION-EDGE` at tick 159 (the tick 243 melee is in cooldown) and is the expected first cinematic; the pileup is the T03 mixed-chain fixture, either order | Reviewer's first option, and the owner's linked melee-plus-shore intent; no new layout needed | Yes: tighten to require the melee kill before the landing if it feels too common |
| F7 drop credit | Match drop-hazard credit through one helper (`DefeatCause::drop_credit()`) covering `ShoreBlock`, `ShoreBlockEdge` and `DropHazard`, a sibling variant with no hash migration | Boss design as merged (#120) | Yes |
| F8 boss | Boss defeats never trigger CINE in v0 | Matches boss design section 5 | Yes |
| F9 pause | Cancel on D6 pause and `Frozen`; no pause-menu wording | No pause menu on main | Yes |
| F10 hit-stop | Pending counts hit-stop in ticks; hit-stop during Active is accepted as dilated | Hit-stop is tick-based | Yes |
| F11 events | Single fan-out in `print_combat_events`, covering all four call sites | Avoids touching the event-log cursor | Yes |
| F12 tests | T09 uses the windowed binary plus the pure parity test; T10 is pure plus manual live check | Headless replays do not run Bevy time | Yes |
| F13 backlog | Table row beside DEATH-D, top section removed; T01/T02 reworded | Backlog format | Yes |
| Phase 1 (a) cooldown | Cooldown starts when the moment is emitted and is counted in sim ticks (adopted default) | Deterministic and independent of presentation timing | Yes |
| Phase 1 (b) upgrade window | The tier upgrade window for a pending moment is 8 ticks (adopted default) | Short fixed window keeps the upgrade deterministic | Yes |
| R3 primaries | Primaries do not stack: one primary (5) plus bonuses; `ShoreBlockEdge` counts as a falling shore block for `MIXED-CHAIN` | Keeps legendary unreachable without `CONTACT-SETUP`; reviewer's recommended default | Yes |
| R4 upgrade window | Upgrades only while `Pending` and within 8 ticks of the first qualifying tick; `Pending` is never extended | Ties phase 1 (b) to the state machine | Yes |
| Phase 1 (c) window bound | "Within 90 ticks" is inclusive (adopted default) | Unambiguous boundary for history and clean windows | Yes |
| Phase 1 (d) input presses | Two presses inside one slowed tick collapse into one, matching the existing input latch (adopted default) | Keeps one edge per fixed tick, no new input semantics | Yes |
| Phase 1 (e) Cooldown state | The Cooldown state lives in the evaluator, not in the planner (adopted default) | Keeps cooldown in the pure, replayable layer | Yes |
| Phase 1 (f) module name | The module is `src/cine.rs` (adopted default) | Implementation naming; all mentions of `cinematic.rs` updated | Yes |

## 8. Implementation status

Implemented behind `--cinematic-finishes[=on|off]` / `FLOORBREAK_CINEMATIC_FINISHES`. It is on for live play and off for scripted fixtures, scenarios, autopilot runs and captures unless set explicitly, so every committed golden is unchanged. Evidence and the T01 to T13 coverage table: [docs/evidence/cine-01](../evidence/cine-01/README.md).

| Piece | Where |
| --- | --- |
| Evaluator, planner, edge latch, tuning table | `src/cine.rs` (Bevy-free) |
| Combat-log adapter: launch proof, per-tick batching, cancel flag | `src/cine_adapter.rs` (Bevy-free) |
| Single fan-out | `print_combat_events` in `src/bin/floor01_scene.rs`; `fixed_step` (the respawn and the step) feeds the observer, the `watchdog` (load, before any tick) and `capture_sequence` (scripted captures, excluded by the spec) pass none |
| Clock and camera | `cine_frame` (`Update`): the planner is the sole owner of the `Time<Virtual>` relative speed; `Time<Fixed>` stays at 60 Hz. Emphasis is added to the scene camera's **kick** only (`SceneCam::emphasis`), never to its zoom or pan |
| Cancel | D6 disconnect pause and `Frozen` (frame context), `Restart`, `Respawn` and `HeroDefeated` (adapter flag), setting switched off, app exit |
| Hit-stop | Pending waits for the fight's hit-stop in ticks; hit-stop starting during Active is accepted (dilated) and never cancels |
| Drop credit | `DefeatCause::drop_credit()` (`ShoreBlock`, `ShoreBlockEdge`, `DropHazard`) |

Choices made while implementing, each changeable in a follow-up:

- The cooldown clock is `epoch + attempt tick`, where a `Restart` adds the tick the finished attempt ended on, so it survives `Restart` and `Respawn` (T07).
- The launch proof credits the launch to hero 0 (`MELEE_ATTACKER`): `Hit` names no attacker and only Carl attacks on main. Replace it when a second attacker exists.
- A `DropHazard` defeat is credited to the player it names and counts as a falling block for `CINE-MIXED-CHAIN`; the boss itself is never mapped (F8), so no v0 chain reaches it.
- The camera emphasis is the planner's constant 2 px for the moment, signed by Carl's facing.
- The planner skips a moment when the death presenter has no unsettled defeat sequence for the moment's target (`DeathPresenter::finish_active`), and when there is no presenter.
- There is no cinematic reduced-motion flag: HUD-DEBT-08 owns world reduced motion and must disable CINE.

