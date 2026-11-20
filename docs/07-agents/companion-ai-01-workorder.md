# COMPANION-AI-01 — Local agent implementation work order

**Status:** Design reconciled with COMBAT-FUN-02 and main; defaults CA-01..CA-40 adopted by the orchestrator (owner design-defaults directive, 2026-11-19); no implementation yet. Game feel and controller UX pending the owner's play.

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
| CA-16 | Hypothesis for 01E under `kill_policy = factual` (the candidate default, CA-19): AI may earn real kills, favours support over hijacking human finisher setups; kill/viewer credit stays factual. Default through 01D is CA-19 `never` |
| CA-17 | HUD overlay appears only while commanding; AI plate has at most one state indicator; true accessibility semantics |
| CA-18 | Do not treat the existing solo 1 HP safeguard as license for reckless AI; no automatic dangerous Carl explosives |
| CA-19 | `kill_policy` per profile set: `never` (default through 01D) or `factual` (the candidate default, tested in 01E, which records a recommendation). Credit is always factual: the hero that actually caused the defeat |
| CA-20 | Glance: an AI-started hit (started by an injected press; the flag survives a swap) whose damage >= target HP deals 0 on its hit tick, keeps the move's non-lethal reaction and logs `Hit` with damage 0; indirect kills are not suppressed, are credited factually and counted as `ai_indirect_kills`; under `never` the utility penalises predicted indirect kills |
| CA-21 | `UncontrolledHero::Companion(CompanionProfileSet)` replaces CF02's `PartyTuning::follower_assist`; sets `cf02_assist` (CF02-Q13..Q17, Q39, Carl passive) and `companion_v0` (both roles) |
| CA-22 | Rollback is `UncontrolledHero::FollowAndJumpBot` (default). CLI `--companion off\|cf02_assist\|v0`; `--cf02` implies `cf02_assist` unless `--companion` is given; with both off every legacy golden is byte-identical |
| CA-23 | Shared perception contract `CompanionView`, owned by COMPANION, built in `src/combat_scene/companion.rs` from pre-tick state, read-only, never hashed; "in view" = `!SceneSim::is_offscreen(x, 0.0)` (CF02-Q40, AP-Q07) |
| CA-24 | The AI acts only by one full `RawInput` per companion-driven hero through `step_party`; v0 never presses Down, pause or swap |
| CA-25 | Director state hashed only while a hero is companion-driven; `CompanionEvent` log separate from `CombatEvent`; decision traces unhashed |
| CA-26 | v0 profiles never bail out (CF02-Q55): Jump is filtered during the AI hero's own move recovery (`ai_bailout: false`) |
| CA-27 | Command mode clears the controlled hero's move buffer and the swap buffer; consumed face presses never reach `input_buffer.rs` |
| CA-28 | `StableEntityId { kind, index, spawn_serial }`; `spawn_serial` per-attempt counter, required from 01D |
| CA-29 | A defeat credited to a companion-driven hero never triggers CINE-01 (adapter passes `slot = None`); AUDIENCE credit stays factual per hero |
| CA-30 | Companion fixtures live in `src/companion_fixtures.rs` (a `PartyFixture` registry) with prefix `companion_`; the moved assistant fixtures keep their `cf02_assist_*` names there; CI reaches both through `--list-fixtures` |
| CA-31 | 01A-01D measure with per-fixture event tallies under AP-Q14 attribution; 01E uses AUTOPLAY-02a; no second harness |
| CA-32 | CF02-Q39 floor (>= 3 AI-started assist moves) and ceiling (<= 25 % of hits) apply per profile set; `companion_v0` reports them per role |
| CA-33 | Asymmetric gates are unimplemented on main: 01B-core builds the graph over existing solids and the camera-lab `donut_only` platforms; gate assertions wait for an authored gate fixture owned by the asymmetric-traversal work |
| CA-34 | 01C is a pure module `src/companion/command_input.rs` plus traces and an optional auto-discovered lab binary; it edits no `input.rs`, `pad_slots.rs`, `party.rs` or `floor01_scene.rs` |
| CA-35 | The AI-plate state icon (01D) avoids the AUDIENCE-01 readout band (+80..+158 x +3..+14 plate-relative) |
| CA-36 | Decision cadence (CA-10, 6 ticks) and action cadence (CF02-Q16, one AI-started move per 60 ticks, none while a move runs) are separate profile knobs |
| CA-37 | v0 AI Carl move set: jab and push kick to interrupt or screen, haymaker only through the legal jab chain; no Down moves, no air moves, no explosives |
| CA-38 | No voice barks or subtitles in 01A-01D; 01D feedback is HUD-only |
| CA-39 | `assets/data/gameplay/companion_ai_v0.json` is loaded, validated and FNV-1a pinned like `companion_magic_v0.json`; a load error names the field; knob names carry their question ID (`ca10_decision_cadence_ticks`, `cf02_q14_swat_tell_tick`, ...). CF02-00 does not create it; 01A-core does |
| CA-40 | 01A movement = `follower_input_boss` plus an approach goal to the chosen target, only for targets in view and within CF02-Q14's 96 px; otherwise follow |

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
| `src/corridor.rs` | CF02-00 (sweep) -> 01A-wire (`PartyStepCtx::companion`, `step_live`) -> C2 (pan block, Q20) -> 01B-wire (nav-safe regroup) |
| `src/party.rs` | CF02-00 (sweep) -> 01A-wire (`UncontrolledHero::Companion(CompanionProfileSet)`) -> C2 (Q19/Q20 knobs) -> 01D (release swap, only after D-CTRL) |
| `src/follower.rs` | CF02-00 (sweep) -> 01A-wire (`pub(crate)` helpers only, if needed) -> 01B-wire. C2 no longer edits it |
| `src/input.rs`, `src/pad_slots.rs` | CF02-00 (`NONE`) -> A (`down`) -> 01D (only if D-CTRL adopts the release swap). 01C edits neither |
| `src/bin/floor01_scene.rs` | CF02-00 (arg block) -> A (input adapter) -> 01A-wire (`--companion` arg) -> F (scenario) -> I (clips) -> 01D (command input, HUD wiring) |
| `scripts/ci_floor01_combat.sh` | CF02-00 (loops over `--list-fixtures cf02_` and `--list-fixtures companion_`), then frozen |
| `src/hud.rs`, `src/hud_draw.rs` | 01D only (no CF02 owner) |
| `src/cf02.rs`, `cf02_combat_v0.json` | CF02 data token; COMPANION read-only |

A PR that finds it must touch a file another running PR owns stops and reports instead of editing it.

## Implementation PRs and dependency graph

| PR | Deliverable | Can start | Ownership / evidence |
|---|---|---|---|
| **01A-core — view, profiles, director, rules** | Bevy-free `CompanionView`, profile sets, director, glance rule, unit tests | **Now**; merges after CF02-00 | `src/companion/**`, `assets/data/gameplay/companion_ai_v0.json`; unit tests, no behaviour change |
| **01A-wire — autonomy foundation** | One real legal combat/support action by AI **Carl and Donut**, preserving follower and safeguard defaults; hooks per CA-24, `UncontrolledHero::Companion`, `--companion`, both-role and `cf02_assist_*` fixtures | After CF02-00, CF02-A, CF02-C1 and 01A-core; runs beside CF02-H | Director/perception/role profiles, both-role headless fixtures, real-event hash parity, rollback flag |
| **01B-core — platform graph** | Pure platform graph, actor-aware path/reachability, danger anchors, destruction invalidation | **Now**, parallel to 01A after interface freeze | Nav/level metadata over existing solids and camera-lab `donut_only` platforms, blocked-path fixtures |
| **01B-wire — nav-driven follow** | Nav-driven follow and camera-safe regroup in `corridor.rs` | After CF02-C2 and 01B-core | Gate coverage once a gate fixture exists, visual capture |
| **01C — controller UX spike** | Tap/hold/chord experiments, alternative binding and input-state-machine tests without production rebinding (pure grammar and traces) | **Now**, parallel | Isolated experimental input harness; real-pad comparison requested |
| **01D — commands + feedback** | Focus, Hold, Regroup, Assist/Act, validator/status, target preview and minimal native HUD | After 01A-wire, the 01B API, owner D-CTRL and CF02-I | One integration owner for input/party/HUD, gamepad + replay + 640 captures |
| **01E — combat synergy + evaluation** | Better positioning, realistic finisher support, individual kill/viewer attribution and metrics; `never` vs `factual` comparison | After 01D, AUTOPLAY stage B and AUTOPLAY-02a | Both-role no-orders/ordered comparison, CPU profiling, human-playtest packet |

**01A-core is the first implementation PR**; 01B-core and 01C may run beside it. 01A-wire is the first PR that changes behaviour, and only under `--companion` or `--cf02`. 01D must not be started as a gameplay integration before the first three foundations have been reconciled. 01E does not need offline ML to prove usefulness. The owner may adjust the staged order after review, but there should never be a speculative giant implementation PR.

**Merged order with COMBAT-FUN-02:**

- **Now:** PR #128 doc fixes; branches for 01A-core (view/profile/director/rules, unit tests), 01B-core (pure platform graph over existing solids and the camera-lab `donut_only` platforms) and 01C (pure grammar `src/companion/command_input.rs` + traces). They touch only COMPANION files and merge once CF02-00's stubs exist.
- **CF02 group 1:** CF02-00 (with COMPANION stubs), ACT-DESIGN.
- **Group 2:** A, C1, CORES (+ art); 01A-core, 01B-core and 01C merge here.
- **Group 3:** H **and 01A-wire** (needs CF02-00, A, C1; disjoint from H's files).
- **Group 4:** B and C2-reduced (needs H, C1, 01A-wire).
- **Group 5:** G, D, AUDIO; 01B-wire may join (needs C2 for `corridor.rs`).
- **Group 6:** F (needs G, C2, 01A-wire; Q39 measured on `cf02_assist`). **7:** I. **8:** REVIEW.
- **After:** 01D (needs 01A-wire, the 01B API, owner D-CTRL on real pads, I released `floor01_scene.rs`); 01E (needs 01D, AUTOPLAY B and AUTOPLAY-02a). The CF02 critical path (00 -> A -> H -> B -> G -> F -> I -> REVIEW) does not lengthen.

### 01A — acceptance

- In a real party fixture with human Carl, AI Donut can safely interrupt/support with a **legal swat**, never cast before unlock; after unlock she obeys the current minimum AI mana reserve.
- In a real party fixture with human Donut, AI Carl can legally protect and attack; **no grabbing, invented bombs or AI-only physics**.
- Fixed-tick traces and state hashes are identical across two runs; existing solo swap, 2P and old combat golden regressions stay intact. Show those metrics for `FollowAndJumpBot` vs the companion on the same fixtures; no commands or new input binding in this PR.
- The eight `cf02_assist_*` fixtures (moved from CF02-C2) pass under `--cf02` with the CF02-Q14..Q17, Q38 (AI part) and Q40 assertions; `cf02_assist_camera_parity` covers `--camera auto` vs `scroll`.
- With `--companion off`, every legacy golden is byte-identical: 20 `EXPECTED_HASHES` (13 legacy, 2 DEATH-D, 5 llama), `GOLDEN_WALK_RIGHT_400`, the story hash `931868d6afa56498`, 7 party, 13 boss and 8 `q10_*` goldens, `demo01_*`, the CINE goldens, and the whole-route autopilot hash `0363078d58558e95`.
- Under `kill_policy = never` no AI-started hit kills; the metrics table lists assist starts, AI hit share, glances and `ai_indirect_kills` per fixture.

### 01B — acceptance

- Traverse ordinary reachable ledges and wait safely at unreachable ones; prove Carl's graph has no edge onto a `donut_only` platform; the Carl-only barrier assertion waits for a gate fixture (CA-33).
- After destruction, only reachable graph edges change; no camera/offscreen teleport bypasses a mandatory gate. Both solo routes remain completable by swapping and no actor becomes permanently trapped.

### 01C — acceptance

- Test ordinary short swap, hold with no selection, fast L1+face chord, overlapping hurt/cooldown/buffer, simultaneous buttons, accessibility toggle, reconnect and transition to two humans.
- One input yields **exactly one** intended semantic action; chord cannot emit player primary/physical/special/jump or also swap. Compare actual switch latency with baseline, include real DualSense/alternate pad result or plainly mark physical validation missing. Keep production binding unchanged until accepted. Traces build inputs with `..RawInput::NONE` and include the CF02-A `down` field; Down never forms a chord; command mode clears the move and swap buffers (CA-27); the module edits no shared input file (CA-34).

### 01D — acceptance

- All four orders produce real deterministic events and status; on no valid target/path/capability, explain refusal without fake completion or unrelated action.
- A single Assist triggers at most one supported action; Hold expires/cancels safely; target entity deletion/new generation cannot alias a stale Focus.
- Switching cancels old AI orders but does not illegally cancel an already committed move; P2 join disables AI and all solo inputs, disconnect still pauses.
- Native Bevy screenshot at 640×360 and 640×400: commands and glyphs legible, no boss tell or action-lane obstruction; no new `bevy_ui` dependency without a separate approved ADR.

### 01E — acceptance

- A verified physical/environmental combo allows AI assistance without treating all kills as player kills; under `kill_policy = factual` the AI may earn normal kills but does not monopolize kill credit, and under `never` its lethal hits become glances.
- Metrics: orders per fight, rejected orders, command latency, useful interrupts, unwanted finisher interceptions, AI hits/resources, stuck time, per-role kill credit; fixtures replay with stable hashes and the existing audience observer reads real events. Runs on AUTOPLAY-02a, comparing `FollowAndJumpBot`, `cf02_assist`, `companion_v0` with `never` and with `factual` on the same seeds; the 01E report records a recommendation for the default kill policy (CA-19); no companion-credited defeat triggers CINE (CA-29); the owner decides D-KILL and D-AUTO.

## AUTOPLAY-02a dependency (01E)

- AUTOPLAY-02a is owned by the AUTOPLAY lead. It is planned after AUTOPLAY stage B, is not part of AUTOPLAY v1, and runs in parallel with AUTOPLAY C to D. 01E depends on 02a, not on a date.
- The mode is `party_solo { seat: 0|1, companion_profile }`, driving `PartyFixtureSim::step_slots`. ActionV1 is reused with swap **reserved**, so the seat stays fixed.
- ObservationV2 adds an ally block that includes Donut's Q10 state and mana.
- Metrics come from AUTOPLAY. Kill credit by hero comes from `EnemyDefeated.instigator`, and companion damage from `Hit` `move_id`. The "assist" metric is AUTOPLAY's definition: a hit on the same enemy id within N ticks before another hero's kill. COMPANION does not define its own assist window. The value of N is a coordination item between COMPANION and AUTOPLAY through the orchestrator and is open.
- 02a code lives in `src/autoplay/party*`. It can start on today's uncontrolled behaviour and switch to `UncontrolledHero::Companion(profile)` once 01A-wire merges.
- Companion-only diagnostics (glances, `ai_indirect_kills`, orders, stuck ticks) come from draining `CompanionEvent`s, offered to 02a as an extra diagnostics source.

## Open owner questions

These need the owner's game feel or real pads; agents do not block on them and record each as open or NOT RUN.

| ID | Question | Default until answered |
| --- | --- | --- |
| O1 (D-CTRL) | Does L1 release-to-swap feel immediate on a DualSense and a Steam Deck under combat pressure? | Production binding unchanged; agents mark it NOT RUN without real pads |
| O2a (D-KILL) | Should the AI land real kills? Answered after 01E's `never` vs `factual` comparison | `kill_policy = never` through 01D; `factual` is the candidate default 01E tests |
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

**First agent task:** implement 01A-core only (Bevy-free view, profiles, director, glance rule, unit tests). It merges after CF02-00's stubs; 01A-wire follows once CF02-A and CF02-C1 have merged.
