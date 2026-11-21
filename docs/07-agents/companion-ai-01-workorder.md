# COMPANION-AI-01 — Local agent implementation work order

**Status:** Design reconciled with COMBAT-FUN-02 and main; defaults CA-01..CA-41 adopted by the orchestrator (owner design-defaults directive, 2026-11-19). **Merged on main (2026-11-20), unwired core modules only:** 01A-core #129 (`e6c8c3b9`), 01B-core #131 (`811dc9b4`) and 01C #130 (`5b343867`); none is wired into a scene, so no behaviour changed and every golden is unchanged. **01A-wire merged via #151 (`78333e3f`, 2026-11-20), game-feel review pending;** its follow-ups, including the staggered-pounce design question for the top tier, are in the [backlog row](../04-production/backlog.md). **01B-wire (nav-driven follow and camera-safe regroup) is in review as a draft PR on `feat/companion-01b-wire`** (see [01B-wire result](#01b-wire-result)); 01D and 01E are not started. Merged, game-feel review pending: game feel and controller UX are pending the owner's play.

**Read:** [Companion autonomy/commands spec](../03-tech/companion-ai-command-system.md), [ADR-0009](../05-decisions/ADR-0009-companion-ai-authority.md), [AGENTS.md](../../AGENTS.md), [local agent protocol](workorders.md), [orchestration](agent-orchestration.md).

## Mission

Make the uncontrolled character in solo (Carl **or** Donut) autonomously useful and trustworthy in combat, traversal and cooperation. Let the human give a small number of intelligible tactical orders without compromising direct hero switching, abilities, local 2P or the combat-first game feel. Deliver in **separate incremental implementation PRs**, each with reproducible proof; do not implement the entire stack in one code PR.

## Current code and concurrent work

- `src/party.rs` owns human/AI authority, solo swaps, cooldown (20 ticks), swap buffer (6 ticks), and current solo safeguards; `src/input.rs` fires swap on **press edge**, not release. Do not migrate binding semantics during AI foundation work. Today the AI Donut cannot start a move (`may_start: !bot` in `src/combat_scene/donut.rs`) and the bot's movement comes from `src/corridor.rs` (`follower_input_boss`).
- `src/follower.rs` is deterministic, handles follow/jumps/tell avoidance and **never attacks**; it has no platform navigation graph. `src/party_scene.rs` has real party replays, `src/combat_scene.rs` contains combat rules; use **these authoritative paths**, not a toy simulator.
- `docs/01-design/mvp-controller-contract.md` preserves Cross/A jump, Square/X primary, Triangle/Y physical, Circle/B special, R1/RB interact, L1/LB swap. Held L1 for orders is reserved, **not approved**. D-pad movement must remain possible, no compulsory right stick or touchpad.
- `docs/01-design/asymmetric-traversal-and-gates.md`: Carl-specific breakable gates and Donut-specific high routes must stay capability-checked; solo switching always provides a valid critical-path solution. No unauthorised teleport through story gates.
- `docs/02-art/hybrid-hud-a-native-spec.md`: existing native 640×360 / 640×400 HUD and AI/human plates. Do not create a parallel UI stack or draw permanent orders over the action lane.
- Donut Q10 swat/pounce/missile rules and mana reserve are already adopted: **AI missile cast only if at least 2 charges remain afterward**; no magic before unlock. Carl cannot grab enemies or use explosives before unlock. Read the reserve through `DonutMana::ai_can_cast` (`companion_magic_v0.json`).
- COMBAT-FUN-02 (#124, merged) and this work order are reconciled (design section 17): this work delivers CF02's restrained assistant as profile `cf02_assist`; CF02-00 lands no-op COMPANION hook stubs; shared files follow the ownership table below. AUTOPLAY-01 v1 stays Carl-only; 01E needs AUTOPLAY-02a (CA-31, see [AUTOPLAY-02a dependency](#autoplay-02a-dependency-01e)). Check merged status immediately before coding.
- The spec is the **complete design proposal**. This work order adds sequencing, interfaces and test obligations rather than superseding its gameplay rules.

## Provisional design decisions for local agents

All numbers are tuning hypotheses pending real gamepad play; record which were adopted in the implementation. Do not ask the owner for ordinary non-critical defaults, and do not declare UX/game feel accepted without tests.

| ID | Default to implement or evaluate |
|---|---|
| CA-01 | A single shared companion director for either uncontrolled hero; capability and utility differences come from character profiles, not separate AI engines |
| CA-02 | Deterministic top-level states and legal utility candidates; safety and existing move commitments veto commands |
| CA-03 | Useful **no-orders** behavior: defensively follow, interrupt imminent danger, limited legal attacks and bounded positioning; kills per CA-19 |
| CA-04 | One persistent order plus one immediate task; no hidden queue; cancel on swap/2P/death/room transitions |
| CA-05 | Four player-facing intents: Focus, Hold, Regroup and contextual Assist/Act. Interact is an Assist variant, not a fifth button |
| CA-06 | Focus on a stable enemy ID until death/inaccessible/cancelled; timeout after ~8s without progress |
| CA-07 | Hold at companion's current *safe* anchor with ~8s normal-room timeout; no point-and-click waypoint in v0 |
| CA-08 | Regroup to the existing follow band without skipping terrain/camera gates |
| CA-09 | Assist/Act executes **one legal action**, never loops spending a resource; contextual interact success must be a real world event |
| CA-10 | Utility planning at ~6 fixed ticks starting cadence, urgent threat/command validation at each fixed tick; stable tie-breakers |
| CA-11 | Capability-aware, risk-scored 2D platform graph, dynamic destruction invalidation and safe no-route response |
| CA-12 | Candidate input UX: L1 short **release** swaps, L1 long (10 ticks) opens command mode, first chord instantly acts; **spike only**, not an adopted gameplay change |
| CA-13 | Proposed chords: L1+Square Focus, +Triangle Hold, +Circle Regroup, +Cross Assist/Act. No leaked attack, jump, cast or swap |
| CA-14 | Alternate R1 modifier/toggle remap must be evaluated if the L1 release delay harms switching; no mandatory D-pad command selection |
| CA-15 | Stable deterministic target preview and explicit refuse/defer/completed reason; never silently pick a different target |
| CA-16 | Hypothesis for 01E under `ca19_kill_policy = factual` (the candidate default, CA-19): AI may earn real kills, favours support over hijacking human finisher setups; kill/viewer credit stays factual. Default through 01D is CA-19 `never` |
| CA-17 | HUD overlay appears only while commanding; AI plate has at most one state indicator; true accessibility semantics |
| CA-18 | Do not treat the existing solo 1 HP safeguard as license for reckless AI; no automatic dangerous Carl explosives |
| CA-19 | `ca19_kill_policy` per profile set: `never` (default through 01D) or `factual` (the candidate default, tested in 01E, which records a recommendation). Credit is always factual: the hero that actually caused the defeat |
| CA-20 | Glance: an AI-started hit (started by an injected press; the flag survives a swap) whose damage >= target HP deals 0 on its hit tick, keeps the move's non-lethal reaction and logs `Hit` with damage 0; indirect kills are not suppressed, are credited factually and counted as `ai_indirect_kills`; under `never` the utility penalises predicted indirect kills, **deferred to the first PR that adds a launching move for the AI, no later than 01E**: the 01A move set (jab, haymaker, swat, pounce, missile) has no launch reaction (`CARL_PUSH_KICK` is the only `launch: true`), so the penalty would be close to empty. It must land together with the wall, hazard and shore data in the view. 01A-wire still tallies `ai_indirect_kills` |
| CA-21 | `UncontrolledHero::Companion(CompanionProfileSet)` replaces CF02's `PartyTuning::follower_assist`; sets `cf02_assist` (CF02-Q13..Q17, Q39, Carl passive) and `companion_v0` (both roles) |
| CA-22 | Rollback is `UncontrolledHero::FollowAndJumpBot` (default). CLI `--companion off\|cf02_assist\|v0`; `--cf02` implies `cf02_assist` unless `--companion` is given; with both off every legacy golden is byte-identical |
| CA-23 | Shared perception contract `CompanionView`, owned by COMPANION, built in `src/combat_scene/companion.rs` from pre-tick state, read-only, never hashed; "in view" = `!SceneSim::is_offscreen(x, 0.0)` (CF02-Q40, AP-Q07) |
| CA-24 | The AI acts only by one full `RawInput` per companion-driven hero through `step_party`; v0 never presses Down, pause or swap |
| CA-25 | Director state hashed only while a hero is companion-driven; `CompanionEvent` log separate from `CombatEvent`; decision traces unhashed |
| CA-26 | v0 profiles never bail out (CF02-Q55): Jump is filtered during the AI hero's own move recovery (`ca26_ai_bailout: false`) |
| CA-27 | Command mode clears the controlled hero's move buffer and the swap buffer; consumed face presses never reach `input_buffer.rs` |
| CA-28 | `StableEntityId { kind, index, spawn_serial }`; `spawn_serial` per-attempt counter, required from 01D |
| CA-29 | A defeat credited to a companion-driven hero never triggers CINE-01 (fail closed: the CINE feed does not forward to `CinematicEvaluator` a `CineEvent::Defeated` whose credited hero, the `instigator` or the cause's `player` for `ShoreBlock`, `ShoreBlockEdge` or `DropHazard`, was companion-driven at that tick; `slot` alone does not gate a moment), owned by 01A-wire; AUDIENCE credit stays factual per hero |
| CA-30 | Companion fixtures live in `src/companion_fixtures.rs` (a `PartyFixture` registry) with prefix `companion_`; the moved assistant fixtures keep their `cf02_assist_*` names there; CI reaches both through `--list-fixtures` |
| CA-31 | 01A-01D measure with per-fixture event tallies under AP-Q14 attribution; 01E uses AUTOPLAY-02a; no second harness |
| CA-32 | CF02-Q39 floor (>= 3 AI-started assist moves) and ceiling (<= 25 % of hits) apply per profile set; `companion_v0` reports them per role (for `cf02_assist` the values are 2 and 50 % since CF02-DUO-A03 and A10) |
| CA-33 | Asymmetric gates are unimplemented on main: 01B-core builds the graph over existing solids and the camera-lab `donut_only` platforms; gate assertions wait for an authored gate fixture owned by the asymmetric-traversal work |
| CA-34 | 01C is a pure module `src/companion/command_input.rs` plus traces and an optional auto-discovered lab binary; it edits no `input.rs`, `pad_slots.rs`, `party.rs` or `floor01_scene.rs` |
| CA-35 | The AI-plate state icon (01D) avoids the AUDIENCE-01 readout band (+80..+158 x +3..+14 plate-relative) |
| CA-36 | Decision cadence (CA-10, 6 ticks) and action cadence (CF02-Q16, one AI-started move per 60 ticks, none while a move runs) are separate profile knobs. A chain follow-up inside the running move's window (haymaker, follow-through kick) is part of the same AI-started move and is not counted |
| CA-37 | v0 AI Carl move set: jab and push kick to interrupt or screen, haymaker only through the legal jab chain; no Down moves, no air moves, no explosives. The push kick is permitted but deferred to the first PR that adds a launching move, no later than 01E, paired with the CA-20 indirect-kill penalty; until then the jab and the jab to haymaker chain meet "legally protect and attack" |
| CA-38 | No voice barks or subtitles in 01A-01D; 01D feedback is HUD-only |
| CA-39 | `assets/data/gameplay/companion_ai_v0.json` is loaded, validated and FNV-1a pinned like `companion_magic_v0.json`; a load error names the field; knob names carry their question ID (`ca10_decision_cadence_ticks`, `cf02_q14_swat_tell_tick`, ...). CF02-00 does not create it; 01A-core does |
| CA-40 | 01A movement = `follower_input_boss` plus an approach goal to the chosen target, only for targets in view and within CF02-Q14's 96 px; otherwise follow. A companion-driven hero starts a move only while both it and its target are in view (Q40, every profile set) |
| CA-41 | Planning hit windows (centre distance): Carl jab/haymaker 6..30 px, Donut swat 6..30 px, conservative subsets of the move box widened by `ENEMY_HALF_WIDTH`. Director knobs in `companion_ai_v0.json` (both profile sets): Carl `ca41_strike_min_px` 6, `ca41_strike_max_px` 30, `ca40_engage_range_px` 96, `ca40_interrupt_range_px` 96; Donut `ca41_swat_hit_min_px` 6, `ca41_swat_hit_max_px` 30 (alongside the CF02-Q14, Q15 and Q17 values, unchanged). A test pins the strike and swat windows (merged with 01A-core #129: `the_planning_hit_windows_stay_inside_the_move_boxes` in `src/companion/profile.rs`) inside `[box.x_min - ENEMY_HALF_WIDTH, box.x_max + ENEMY_HALF_WIDTH]` so a box change cannot drift silently. Q17 missile timing is an arrival-time gate, not a knob (no `ca40_missile_latest_tell_tick` exists): `tell_tick + MAGIC_MISSILE.startup + ceil((distance - ENEMY_HALF_WIDTH) / projectile speed) < tell_len`, refined by 01A-core to the flight model in section 17 (c) with `bolt_lead_px` |

## Interfaces to freeze before parallel coding

The integrator first defines a concise Bevy-free API in a small contract PR or stable documented design decision. Exact Rust names can follow local conventions. Keep physics and collisions in the existing simulation.

- **World observation:** `CompanionView`, design section 17 (CA-23).
- **Navigation query:** `can_reach(actor, source, destination, unlocks)`, `safe_anchor(actor, context)`, `path_or_reason(actor, anchors)`; stable anchor IDs and revision invalidation on destruction.
- **Command intent:** timestamp/tick + sequence + issuer + recipient + verb + stable target/anchor if applicable. Issuer is the human at issuance; recipient is always the currently uncontrolled solo actor.
- **Order results:** accepted, rejected, deferred, executing, completed, cancelled plus typed reason. HUD/subtitles derive from these events, not separately invented UI state.
- **Policy choice:** one full `RawInput` per companion-driven hero through the press-injection boundary in design section 17 (CA-24), respecting cooldowns, movement, mana, collision and recovery. A threat can interrupt only if the underlying combat rules permit it.
- **Event/replay:** version/hash policy tuning and all gameplay-authoritative order state and stable target IDs; ordered action, combat, assist and kill attribution. Presentation-only previews do not affect gameplay hashes. See CA-25.

### Shared files and who owns them when

One owner at a time (merged with the COMBAT-FUN-02 table).

| File | Sequence |
| --- | --- |
| `src/lib.rs` | CF02-00 only (adds `pub mod companion;`, `pub mod companion_fixtures;` with the CF02 lines); AUTOPLAY A's one line rebases |
| `src/combat_scene.rs` | CF02-00 (sweep + COMPANION stubs: `mod companion;`, `companion` field, restart call, hash call, input hook, Carl strike glance hook) -> A -> H -> B -> G -> F. COMPANION never edits it afterwards; a missing hook is a one-line token from the current owner |
| `src/combat_scene/companion.rs`, `src/companion/**`, `src/companion_fixtures.rs`, `assets/data/gameplay/companion_ai_v0.json` | CF02-00 (empty stubs; the json is created by 01A-core) -> COMPANION only |
| `src/combat_scene/donut.rs` | CF02-00 (sweep) -> C1 -> 01A-wire (`may_start`, glance at hit sites) -> C2 (bail-out C6) |
| `src/donut_fixtures.rs` | CF02-00 -> C1 -> C2 (`cf02_donut_bailout`); COMPANION uses `companion_fixtures.rs` |
| `src/corridor.rs` | CF02-00 (sweep) -> 01A-wire (`step_live` companion branch) -> C2 (pan block, Q20) -> 01B-wire (nav-safe regroup) |
| `src/party.rs` | CF02-00 (sweep) -> 01A-wire (`UncontrolledHero::Companion(CompanionProfileSet)`) -> C2 (Q19/Q20 knobs) -> 01D (release swap, only after D-CTRL) |
| `src/follower.rs` | CF02-00 (sweep) -> 01A-wire (`pub(crate)` helpers only, if needed) -> 01B-wire. C2 no longer edits it |
| `src/input.rs`, `src/pad_slots.rs` | CF02-00 (`NONE`) -> A (`down`) -> 01D (only if D-CTRL adopts the release swap). 01C edits neither |
| `src/bin/floor01_scene.rs` | CF02-00 (arg block) -> A (input adapter) -> 01A-wire (`--companion` arg) -> F (scenario) -> I (clips) -> 01D (command input, HUD wiring) |
| `scripts/ci_floor01_combat.sh` | CF02-00 (loops over `--list-fixtures cf02_` and `--list-fixtures companion_`), then frozen |
| `src/cine_adapter.rs` (feed filter only) | 01A-wire |
| `src/hud.rs`, `src/hud_draw.rs` | 01D only (no CF02 owner) |
| `src/cf02.rs`, `cf02_combat_v0.json` | CF02 data token; COMPANION read-only |

A PR that finds it must touch a file another running PR owns stops and reports instead of editing it.

## Implementation PRs and dependency graph

| PR | Deliverable | Can start | Ownership / evidence |
|---|---|---|---|
| **01A-core — view, profiles, director, rules** | Bevy-free `CompanionView`, profile sets, director, glance rule, unit tests | **Merged**, #129 (`e6c8c3b9`), after CF02-00 | `src/companion/**`, `assets/data/gameplay/companion_ai_v0.json`; unit tests, no behaviour change |
| **01A-wire — autonomy foundation** | One real legal combat/support action by AI **Carl and Donut**, preserving follower and safeguard defaults; hooks per CA-24, `UncontrolledHero::Companion`, `--companion`, the CA-29 CINE filter, both-role and `cf02_assist_*` fixtures | After CF02-00, CF02-A, CF02-C1 and 01A-core; runs beside CF02-H | Director/perception/role profiles, both-role headless fixtures, real-event hash parity, rollback flag |
| **01B-core — platform graph** | Pure platform graph, actor-aware path/reachability, danger anchors, destruction invalidation | **Merged**, #131 (`811dc9b4`) | Nav/level metadata over existing solids and camera-lab `donut_only` platforms, blocked-path fixtures |
| **01B-wire — nav-driven follow** | Nav-driven follow and camera-safe regroup in `corridor.rs` | **In review** (draft PR, `feat/companion-01b-wire`); after CF02-C2 and 01B-core, both merged | Gate coverage once a gate fixture exists, visual capture (done, see [01B-wire result](#01b-wire-result)) |
| **01C — controller UX spike** | Tap/hold/chord experiments, alternative binding and input-state-machine tests without production rebinding (pure grammar and traces) | **Merged**, #130 (`5b343867`) | Isolated experimental input harness; real-pad comparison requested |
| **01D — commands + feedback** | Focus, Hold, Regroup, Assist/Act, validator/status, target preview and minimal native HUD | After 01A-wire, the 01B API, owner D-CTRL and CF02-I | One integration owner for input/party/HUD, gamepad + replay + 640 captures |
| **01E — combat synergy + evaluation** | Better positioning, realistic finisher support, individual kill/viewer attribution and metrics; `never` vs `factual` comparison | After 01D, AUTOPLAY stage B and AUTOPLAY-02a | Both-role no-orders/ordered comparison (02a serves the no-orders half only, see the 02a dependency section), CPU profiling, human-playtest packet |

**01A-core, 01B-core and 01C are merged (2026-11-20) as unwired core modules** (`src/companion/{director,view,profile,rules,nav,command_input}.rs`; `src/companion/mod.rs` states that nothing is wired into a scene yet); per AGENTS.md they are recorded as merged, game-feel review pending. 01A-wire is the first PR that changes behaviour, and only under `--companion` or `--cf02`. 01D must not be started as a gameplay integration before the first three foundations have been reconciled. 01E does not need offline ML to prove usefulness. The owner may adjust the staged order after review, but there should never be a speculative giant implementation PR.

**Merged order with COMBAT-FUN-02:**

- **Done (merged 2026-11-20 after CF02-00's stubs, #134):** 01A-core #129 (view/profile/director/rules, unit tests), 01B-core #131 (pure platform graph over existing solids and the camera-lab `donut_only` platforms) and 01C #130 (pure grammar `src/companion/command_input.rs` + traces). They touch only COMPANION files.
- **CF02 group 1:** CF02-00 (with COMPANION stubs), ACT-DESIGN.
- **Group 2:** A, C1, CORES (+ art); 01A-core, 01B-core and 01C were merged here ahead of schedule (2026-11-20).
- **Group 3:** A+H **and 01A-wire** (needs CF02-00, A, C1; disjoint from H's files). Speed mode groups CF02-A+H into one PR (`feat/cf02-ah`): 01A-wire starts stacked on it as soon as the CF02-A commits are pushed there and merges right after A+H.
- **Group 4:** B (grouped with G as B+G in speed mode) and C2-reduced (needs H, C1, 01A-wire).
- **Group 5:** G, D, AUDIO; 01B-wire may join (needs C2 for `corridor.rs`).
- **Group 6:** F (needs G, C2, 01A-wire; Q39 measured on `cf02_assist`). **7:** I. **8:** REVIEW.
- **After:** 01D (needs 01A-wire, the 01B API, owner D-CTRL on real pads, I released `floor01_scene.rs`); 01E (needs 01D, AUTOPLAY B and AUTOPLAY-02a). With speed-mode grouping the real CF02 critical path is 00 -> CORES -> A+H -> 01A-wire -> C2 -> F -> I -> REVIEW (see [the CF02 work order](combat-fun-02-workorder.md#parallel-groups)); 01A-wire is developed stacked on `feat/cf02-ah` so that it merges right after A+H, which keeps the lengthening to that one merge.

### 01A — acceptance

- In a real party fixture with human Carl, AI Donut can safely interrupt/support with a **legal swat**, never cast before unlock; after unlock she obeys the current minimum AI mana reserve.
- In a real party fixture with human Donut, AI Carl can legally protect and attack; **no grabbing, invented bombs or AI-only physics**.
- Fixed-tick traces and state hashes are identical across two runs; existing solo swap, 2P and old combat golden regressions stay intact. Show those metrics for `FollowAndJumpBot` vs the companion on the same fixtures; no commands or new input binding in this PR.
- An AI indirect kill triggers no CINE moment (CA-29 CINE filter).
- The eight `cf02_assist_*` fixtures (moved from CF02-C2) pass under `--cf02` with the CF02-Q14..Q17, Q38 (AI part) and Q40 assertions; `cf02_assist_camera_parity` covers `--camera auto` vs `scroll`.
- With `--companion off`, every legacy golden is byte-identical: 20 `EXPECTED_HASHES` (13 legacy, 2 DEATH-D, 5 llama), `GOLDEN_WALK_RIGHT_400`, the story hash `931868d6afa56498`, 7 party, 13 boss and 8 `q10_*` goldens, `demo01_*`, the CINE goldens, and the whole-route autopilot hash `0363078d58558e95`.
- Under `ca19_kill_policy = never` no AI-started hit kills (profile `companion_v0`; profile `cf02_assist` is `factual` since CF02-DUO-A01, with the anti-kill-steal window of CF02-DUO-A08, spec C-11); the metrics table lists assist starts, AI hit share, glances and `ai_indirect_kills` per fixture.

### 01B — acceptance

- Traverse ordinary reachable ledges and wait safely at unreachable ones; prove Carl's graph has no edge onto a `donut_only` platform; the Carl-only barrier assertion waits for a gate fixture (CA-33).
- After destruction, only reachable graph edges change; no camera/offscreen teleport bypasses a mandatory gate. Both solo routes remain completable by swapping and no actor becomes permanently trapped.

### 01A-core, 01B-core review follow-ups (batch 1)

Recorded from the independent reviews of the 01A-core (#129) and 01B-core (#131) PRs, both now merged; no adopted value changes. Names such as `ai_indirect_kills` and `CompanionEvent` below are not in the code yet: they belong to the 01A-wire and 01E work.

**Deferred to 01E (or earlier where stated):**

- The CA-20 indirect-kill utility penalty and the AI Carl push kick (CA-37): deferred to the first PR that adds a launching move, no later than 01E, landing together with the wall, hazard and shore data in the view.
- Boss play (`view.boss` boss tells) and the human-finisher anti-steal window (only needed under `ca19_kill_policy = factual`, spec section 8): 01E.
- `empty_press: swat` after the unlock (Primary is routed to the cast whenever `unlocked && can_start_cast`): 01E.

**01A-wire obligations (brief notes):**

1. Fill `CompanionView.follow` with `follower_input_boss` on the arguments `step_live` uses, pre-tick; `ThreatView.tell_len` (goblin 24, llama 36); `ManaView.projectile_speed_px_per_tick` from `donut_kit.tuning`; and `ManaView.bolt_lead_px` (35).
2. The bolt constants in `src/combat_scene/donut.rs` are private (the module is private). Add a pin test that the builder's `bolt_lead_px` equals `BOLT_SPAWN_FORWARD + BOLT_BOX_AHEAD + BOLT_BOX_W / 2`; it may export `BOLT_LEAD_PX` from `combat_scene` for that.
3. Pin the Q14 and CA-36 boundaries with a real party fixture: the view is pre-tick, the heroes step and then the goblins, and the move clock advances before a press is read. A swat or jab press must hit at attack tick `< tell_len`; a chain press is judged at `elapsed + 1` and must be `< CARL_JAB.total()`.
4. A chained haymaker is a new `MoveRunner` instance and must also be tagged `ai_started`. `Glance` gains a `policy` field and `apply` calls `rules::glance(*spec, hp, ai_started, policy)`.
5. Mix `PROFILE_HASH` and the set id into the scene hash while a hero is companion-driven (spec section 10 step 7).
6. Confirm on the next tick that `active_move.ai_started` is set and tally unconfirmed presses as a `CompanionEvent`, with a rollback of `last_action_tick` if fixtures show any.
7. Every `RawInput` literal in `src/companion` ends in `..RawInput::NONE` (done on main with #129, `e6c8c3b9`); when CF02-A adds `RawInput::down`, the director leaves it false in v0 (spec section 17 (c), item 1).

**01B-wire obligations:**

1. The nav model and its replay start every transit from rest at the launch x (`EdgeView::launch_x`). The executor must reach that x and stop before it presses jump, or replay the transit arriving at run speed.
2. `NavConfig::integrated()` is the default and the config 01B-wire uses (`JUMP_SPEED`/`RUN_SPEED`); `NavConfig::camera_lab()` is for lab tests only and is not replay-validated (the lab body is not `step_hero`).

### 01B-wire result

Both obligations above are met. Code: `src/companion/navfollow.rs` (pure executor and regroup chooser, `NavConfig::integrated()`), `src/combat_scene/companion.rs` (graph sync, regroup before the view, the nav-driven follow layer), `src/corridor.rs` (`regroup_due`, `regroup_place`, `regroup_skip`, `dodge_input`, the `nav_regroup` control flag), `src/companion/nav.rs` (`edge_plan`, `solid_count`). Nothing changes without `UncontrolledHero::Companion`.

- **Executor (obligation 1).** A hero on one anchor whose route to the leader's anchor contains a jump or a drop commits to the first edge: it steers (analog, the nav replay's steering) to the edge's `launch_x` and presses jump only once it is within 0.6 px of it (the stick dead zone leaves up to 0.55 px), then steers to the edge's target x, staying beside the ledge until its feet are above it. A press that does not take is retried after a release tick; a transit is abandoned when its edge closes (a destruction) or after three times its ETA plus 120 ticks.
- **No route (wait safely).** `plan_bounded` names a safe anchor inside the camera (24 px margin); the hero stands there, closing in on the leader's x within the bot's 40 px band without leaving the anchor, and reports the reason once.
- **Regroup.** After the legacy `regroup_ticks` off-screen, a companion-driven hero is placed on an anchor it could reach by itself, from which the leader is still reachable, just outside the view on the side behind the leader, feet on the anchor (not on the leader's foot line). Otherwise it stays and a `Regroup(Denied)` event is the camera warning; a hero standing on no known anchor falls back to the legacy placement (`Regroup(Legacy)`). The placement happens before the follow layer is read, which closes the 01A one-tick lag after a regroup teleport. The remaining difference to the bot is a goblin tell that Carl's same-tick strike interrupts: the companion dodges it for that one tick (documented at the `step_live` companion branch).
- **Evidence.** `companion_arena_nav_perch_donut` (AI Donut under the left perch slab, whose underside is 68 px up, reaches the slab by one committed jump; the follow-and-jump bot walks under it and bumps its head, feet 2.9 px up at the end) and `companion_arena_nav_perch_carl` (the same with Donut controlled and AI Carl). Replays: `docs/evidence/companion-ai/headless_companion_arena_nav_perch_*.txt`; images: [`docs/evidence/companion-ai/01b-wire/`](../evidence/companion-ai/01b-wire/).
- **Not done.** No gate fixture exists (CA-33), so no Carl-only barrier assertion; the nav level has no hazards and no unlock data (`Unlocks::NONE`); the camera-lab physics are not used.

**Adopted defaults (01B-wire; the owner can change any in a follow-up PR):**

| ID | Default |
|---|---|
| NW-01 | The nav replaces the bot's follow input only when the route needs a jump or a drop, or no route exists; on one anchor, a walking route, an airborne leader or a hero on no anchor the bot's follow stays (flat ground replays are unchanged) |
| NW-02 | The bot's goblin and boss dodges win over the nav's movement (the bot is asked with itself as the leader) |
| NW-03 | Wait anchors must lie 24 px inside the camera; the waiting hero keeps the bot's 40 px follow band and 8 px slack |
| NW-04 | The regroup keeps the legacy trigger (`regroup_ticks` off-screen), the behind-the-leader side and the off-screen test; a candidate must be reachable by the hero and keep the leader reachable; the nearest to the leader's height, then x, wins |
| NW-05 | A refused regroup is a camera warning (`CompanionEvent::Regroup`, `regroups_denied`), not a teleport; a hero on no anchor uses the legacy rule so nobody is stranded |
| NW-06 | The graph is built per attempt from the scene's solids and synced every tick; only a live transit is hashed (a run that never leaves its anchor hashes as before) |
| NW-07 | Arena room fixtures of COMPANION are named `companion_arena_*`; `cf02::arena::fixture_uses_arena` gets that one prefix (a one-line edit of a CF02 file) |

### 01C — acceptance

- Test ordinary short swap, hold with no selection, fast L1+face chord, overlapping hurt/cooldown/buffer, simultaneous buttons, accessibility toggle, reconnect and transition to two humans.
- One input yields **exactly one** intended semantic action; chord cannot emit player primary/physical/special/jump or also swap. Compare actual switch latency with baseline, include real DualSense/alternate pad result or plainly mark physical validation missing. Keep production binding unchanged until accepted. Traces build inputs with `..RawInput::NONE` and include the CF02-A `down` field; Down never forms a chord; command mode clears the move and swap buffers (CA-27); the module edits no shared input file (CA-34).

### 01D — acceptance

- All four orders produce real deterministic events and status; on no valid target/path/capability, explain refusal without fake completion or unrelated action.
- A single Assist triggers at most one supported action; Hold expires/cancels safely; target entity deletion/new generation cannot alias a stale Focus.
- Switching cancels old AI orders but does not illegally cancel an already committed move; P2 join disables AI and all solo inputs, disconnect still pauses.
- Native Bevy screenshot at 640×360 and 640×400: commands and glyphs legible, no boss tell or action-lane obstruction; no new `bevy_ui` dependency without a separate approved ADR.

### 01E — acceptance

- A verified physical/environmental combo allows AI assistance without treating all kills as player kills; under `ca19_kill_policy = factual` the AI may earn normal kills but does not monopolize kill credit, and under `never` its lethal hits become glances.
- Metrics: orders per fight, rejected orders, command latency, useful interrupts, unwanted finisher interceptions, AI hits/resources, stuck time, per-role kill credit; fixtures replay with stable hashes and the existing audience observer reads real events. Runs on AUTOPLAY-02a, comparing `FollowAndJumpBot`, `cf02_assist`, `companion_v0` with `never` and with `factual` on the same seeds; the 01E report records a recommendation for the default kill policy (CA-19); no companion-credited defeat triggers CINE (CA-29); the owner decides D-KILL and D-AUTO.

## AUTOPLAY-02a dependency (01E)

- AUTOPLAY-02a is owned by the AUTOPLAY lead. It is planned after AUTOPLAY stage B, is not part of AUTOPLAY v1, and runs in parallel with AUTOPLAY C to D. 01E depends on 02a, not on a date.
- The mode is `party_solo { seat: 0|1, companion_profile }`, driving `PartyFixtureSim::step_slots`. ActionV1 is reused with swap **reserved**, so the seat stays fixed.
- ObservationV2 adds an ally block that includes Donut's Q10 state and mana.
- Metrics come from AUTOPLAY. Kill credit by hero comes from `EnemyDefeated.instigator`, and companion damage from `Hit` `move_id`. The "assist" metric is AUTOPLAY's definition: a hit on the same enemy id within N ticks before another hero's kill. COMPANION does not define its own assist window. The value of N is **90 ticks**, set by AUTOPLAY AP-Q31.
- 02a code lives in `src/autoplay/party*`. It can start on today's uncontrolled behaviour and switch to `UncontrolledHero::Companion(profile)` once 01A-wire merges.
- Companion-only diagnostics (glances, `ai_indirect_kills`, orders, stuck ticks) come from draining `CompanionEvent`s, offered to 02a as an extra diagnostics source.

Obligations recorded by the AUTOPLAY-02a design ([AUTOPLAY spec section 11](../03-tech/autoplay-01-automated-gameplay-evaluation.md#11-autoplay-02a--solo-party-env-one-policy-seat-companion-active), PR #136):

- The seat is serialized by hero name (`carl`, `donut`; AP-Q22), with 0 and 1 as its numeric value.
- **Kill-policy selector (conflict 2):** 01A-wire or 01E owns it (two profile sets or a pub override, so `companion_v0` runs with `never` and `factual` on one build). The PR that lands it also enables `companion_v0_factual` in the AUTOPLAY registry in `src/autoplay/party/scenario.rs` (one line, approved by the coordinator).
- **01E's ordered half is not served by 02a (conflict 4):** 02a is no-orders only because L1 is its reserved, stripped swap index. 01E reports no-orders numbers from 02a and claims no ordered comparison from it.
- **Rules context (conflict 5):** all 01E arms run under one `cf02_rules` value, `false` until CF02 exposes the switch on the party fixture path. `cf02_assist` with `cf02_rules` off is outside the CA-22 flag table; 01E re-runs all four arms under `true` once the switch exists.
- **Swap timing:** if the owner's D-CTRL decision adopts the release swap (ADR-0009 item 4), the swap-timing PR adds `PartyState::new_solo_controlled(tuning, hero)` in `src/party.rs` and switches 02a to it (AP-Q34).
- Any PR that adds a `PartyTuning` field also extends the `party_tuning_hash` encoder in `src/autoplay/party/` and re-pins its hash test (AP-Q24, AP-Q34); the coordinator sequences it with the `src/party.rs` chain.

## Open owner questions

These need the owner's game feel or real pads; agents do not block on them and record each as open or NOT RUN.

| ID | Question | Default until answered |
| --- | --- | --- |
| O1 (D-CTRL) | Does L1 release-to-swap feel immediate on a DualSense and a Steam Deck under combat pressure? | Production binding unchanged; agents mark it NOT RUN without real pads |
| O2a (D-KILL) | Should the AI land real kills? Answered after 01E's `never` vs `factual` comparison | `ca19_kill_policy = never` through 01D; `factual` is the candidate default 01E tests |
| O2b (D-AUTO) | Is AI Carl active (`companion_v0`) or passive (`cf02_assist`) in the arena the owner plays? | `--cf02` = `cf02_assist` |
| O3 | May a companion-credited defeat ever earn a CINE-01 moment? | No (CA-29) |
| O4 (D-HOLD) | Are four orders useful, or does Hold merge into Regroup/Focus? | Four orders |
| O5 (D-CONTEXT) | Which authored objects may the AI operate without weakening the switch mechanic? | Authored `ContextAct` only |
| O6 (D-FOCUS) | Is the deterministic candidate preview precise enough in dense rooms? | Candidate preview |
| O7 | Real two-pad hot-join with a pending order (C-13 live check) | NOT RUN until two physical pads are available |


## Cross-stage regression suite

Implement/fix specific named fixtures from [the design spec](../03-tech/companion-ai-command-system.md#12-representative-tests-headless-first): C-01 through C-16. Keep existing `solo_swap_continuity`, `solo_follower_dodge`, `solo_guard_floors_at_one`, `duo_independent_moves`, Donut mana/unlock, `q10_*`, boss and (once landed) `cf02_*` fixtures intact; the full golden inventory is in 01A acceptance. Extend them only where the real simulator supports a scenario. Do not replace the simulator, regenerate old goldens blindly or mark a docs-only design proposal as a tested feature.

Baseline commands when relevant:

```sh
python3 scripts/check_docs.py
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
bash scripts/ci_floor01_combat.sh headless
# available after CF02-00
cargo run --locked --bin floor01_scene -- --list-fixtures companion_
```

Use actual, currently supported script flags. Supply replay hashes, ordered event traces, real rendering captures for visual changes and CPU cost for AI planning. If a tool or hardware device is missing, report the exact blocker; **do not fabricate check results**.

## PR description and review handoff

Each stage PR includes: **Why**, **Change**, **Acceptance Criteria** (checked only when verified), **Commands & Results**, **Visual Evidence**, **Performance Data**, **Rights/Provenance**, **Risks**, **Next PR**. An independent gameplay/architecture reviewer checks the final head; subsequent merged mechanics stay **game-feel review pending** until the owner plays. The coordinator updates the backlog status per stage; do not silently mark all COMPANION-AI-01 work complete after 01A.

**Next agent task:** 01A-core, 01B-core, 01C and 01A-wire (#151) are merged; 01B-wire is in review (draft PR); COMBAT-LAB-03 (both heroes traversing tiers with AI) builds on it.
