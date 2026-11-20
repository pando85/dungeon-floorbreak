# COMPANION-AI-01 — Local agent implementation work order

**Status:** Documentation-only handoff, not an implemented gameplay feature. Gameplay tuning and controller UX remain pending real-player review.

**Read:** [Companion autonomy/commands spec](../03-tech/companion-ai-command-system.md), [ADR-0009](../05-decisions/ADR-0009-companion-ai-authority.md), [AGENTS.md](../../AGENTS.md), [local agent protocol](workorders.md), [orchestration](agent-orchestration.md).

## Mission

Make the uncontrolled character in solo (Carl **or** Donut) autonomously useful and trustworthy in combat, traversal and cooperation. Let the human give a small number of intelligible tactical orders without compromising direct hero switching, abilities, local 2P or the combat-first game feel. Deliver in **separate incremental implementation PRs**, each with reproducible proof; do not implement the entire stack in one code PR.

## Current code and concurrent work

- `src/party.rs` owns human/AI authority, solo swaps, cooldown (20 ticks), swap buffer (6 ticks), and current solo safeguards; `src/input.rs` fires swap on **press edge**, not release. Do not migrate binding semantics during AI foundation work.
- `src/follower.rs` is deterministic, handles follow/jumps/tell avoidance and **never attacks**; it has no platform navigation graph. `src/party_scene.rs` has real party replays, `src/combat_scene.rs` contains combat rules; use **these authoritative paths**, not a toy simulator.
- `docs/01-design/mvp-controller-contract.md` preserves Cross/A jump, Square/X primary, Triangle/Y physical, Circle/B special, R1/RB interact, L1/LB swap. Held L1 for orders is reserved, **not approved**. D-pad movement must remain possible, no compulsory right stick or touchpad.
- `docs/01-design/asymmetric-traversal-and-gates.md`: Carl-specific breakable gates and Donut-specific high routes must stay capability-checked; solo switching always provides a valid critical-path solution. No unauthorised teleport through story gates.
- `docs/02-art/hybrid-hud-a-native-spec.md`: existing native 640×360 / 640×400 HUD and AI/human plates. Do not create a parallel UI stack or draw permanent orders over the action lane.
- Donut Q10 swat/pounce/missile rules and mana reserve are already adopted: **AI missile cast only if at least 2 charges remain afterward**; no magic before unlock. Carl cannot grab enemies or use explosives before unlock.
- At authoring (2026-11-20), **AUTOPLAY-01 PR #127** is open, implementing a **Carl-only** first stage. **COMBAT-FUN-02 PR #124** is also open and changes combat priorities. Check actual merged status immediately before coding; avoid changing their shared files concurrently. AUTOPLAY-01's eventual Gymnasium work is optional downstream; it is **not** a dependency of companion AI v0.
- The supplied 40k-character spec is the **complete design proposal**. This work order adds sequencing, interfaces and test obligations rather than superseding its gameplay rules.

## Provisional design decisions for local agents

All numbers are tuning hypotheses pending real gamepad play; record which were adopted in the implementation. Do not ask the owner for ordinary non-critical defaults, and do not declare UX/game feel accepted without tests.

| ID | Default to implement or evaluate |
|---|---|
| CA-01 | A single shared companion director for either uncontrolled hero; capability and utility differences come from character profiles, not separate AI engines |
| CA-02 | Deterministic top-level states and legal utility candidates; safety and existing move commitments veto commands |
| CA-03 | Useful **no-orders** behavior: defensively follow, interrupt imminent danger, limited legal attacks, bounded positioning and normal kills |
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
| CA-16 | AI may earn real spectacular kills, but should favor support over hijacking human finisher setups; kill/viewer credit stays factual |
| CA-17 | HUD overlay appears only while commanding; AI plate has at most one state indicator; true accessibility semantics |
| CA-18 | Do not treat the existing solo 1 HP safeguard as license for reckless AI; no automatic dangerous Carl explosives |

## Interfaces to freeze before parallel coding

The integrator first defines a concise Bevy-free API in a small contract PR or stable documented design decision. Exact Rust names can follow local conventions. Keep physics and collisions in the existing simulation.

- **World observation:** read-only current actor, leader, enemies, active tells, capabilities, resources, phase, platforms, hazards and camera bounds; no future inputs or mutable world internals.
- **Navigation query:** `can_reach(actor, source, destination, unlocks)`, `safe_anchor(actor, context)`, `path_or_reason(actor, anchors)`; stable anchor IDs and revision invalidation on destruction.
- **Command intent:** timestamp/tick + sequence + issuer + recipient + verb + stable target/anchor if applicable. Issuer is the human at issuance; recipient is always the currently uncontrolled solo actor.
- **Order results:** accepted, rejected, deferred, executing, completed, cancelled plus typed reason. HUD/subtitles derive from these events, not separately invented UI state.
- **Policy choice:** legal desired action expressed using the existing semantic action path/RawInput, respecting cooldowns, movement, mana, collision and recovery. A threat can interrupt only if the underlying combat rules permit it.
- **Event/replay:** version/hash policy tuning and all gameplay-authoritative order state and stable target IDs; ordered action, combat, assist and kill attribution. Presentation-only previews do not affect gameplay hashes.

One integrator at a time owns changes to shared `src/input.rs`, `src/party.rs`, `src/combat_scene.rs`, `src/bin/floor01_scene.rs`, and `src/hud.rs`. Other agents own distinct new modules and pass narrow interfaces; do not edit unmerged changes in a shared worktree.

## Implementation PRs and dependency graph

| PR | Deliverable | Can start | Ownership / evidence |
|---|---|---|---|
| **01A — autonomy foundation** | One real legal combat/support action by AI **Carl and Donut**, preserving follower and safeguard defaults | **Now** | Director/perception/role profiles, both-role headless fixtures, real-event hash parity, rollback flag |
| **01B — traversal/navigation** | Platform graph, actor-aware path/reachability, danger anchors, destruction invalidation, camera-safe regroup | **Parallel to 01A after interface freeze** | Nav/level metadata, E1 gates, blocked-path fixtures, visual capture |
| **01C — controller UX spike** | Tap/hold/chord experiments, alternative binding and input-state-machine tests without production rebinding | **Parallel** | Isolated experimental input harness; real-pad comparison requested |
| **01D — commands + feedback** | Focus, Hold, Regroup, Assist/Act, validator/status, target preview and minimal native HUD | **After 01A, nav API and decision from 01C** | One integration owner for input/party/HUD, gamepad + replay + 640 captures |
| **01E — combat synergy + evaluation** | Better positioning, realistic finisher support, individual kill/viewer attribution and metrics | **After 01D** | Both-role no-orders/ordered comparison, CPU profiling, human-playtest packet |

**01A is the first implementation PR**, even if 01B/01C experiments are delegated immediately. 01D must not be started as a gameplay integration before the first three foundations have been reconciled. 01E does not need offline ML to prove usefulness. The owner may adjust the staged order after review, but there should never be a speculative giant implementation PR.

### 01A — acceptance

- In a real party fixture with human Carl, AI Donut can safely interrupt/support with a **legal swat**, never cast before unlock; after unlock she obeys the current minimum AI mana reserve.
- In a real party fixture with human Donut, AI Carl can legally protect and attack; **no grabbing, invented bombs or AI-only physics**.
- Fixed-tick traces and state hashes are identical across two runs; existing solo swap, 2P and old combat golden regressions stay intact. Show before/after no-orders metrics; no commands or new input binding in this PR.

### 01B — acceptance

- Traverse ordinary reachable ledges and wait safely at unreachable ones; prove Carl cannot follow a Donut-only perch and Donut cannot destroy a Carl-only barrier.
- After destruction, only reachable graph edges change; no camera/offscreen teleport bypasses a mandatory gate. Both solo routes remain completable by swapping and no actor becomes permanently trapped.

### 01C — acceptance

- Test ordinary short swap, hold with no selection, fast L1+face chord, overlapping hurt/cooldown/buffer, simultaneous buttons, accessibility toggle, reconnect and transition to two humans.
- One input yields **exactly one** intended semantic action; chord cannot emit player primary/physical/special/jump or also swap. Compare actual switch latency with baseline, include real DualSense/alternate pad result or plainly mark physical validation missing. Keep production binding unchanged until accepted.

### 01D — acceptance

- All four orders produce real deterministic events and status; on no valid target/path/capability, explain refusal without fake completion or unrelated action.
- A single Assist triggers at most one supported action; Hold expires/cancels safely; target entity deletion/new generation cannot alias a stale Focus.
- Switching cancels old AI orders but does not illegally cancel an already committed move; P2 join disables AI and all solo inputs, disconnect still pauses.
- Native Bevy screenshot at 640×360 and 640×400: commands and glyphs legible, no boss tell or action-lane obstruction; no new `bevy_ui` dependency without a separate approved ADR.

### 01E — acceptance

- A verified physical/environmental combo allows AI assistance without treating all kills as player kills; single-player AI may earn normal/spectacular kills but does not monopolize kill credit.
- Metrics: orders per fight, rejected orders, command latency, useful interrupts, unwanted finisher interceptions, AI hits/resources, stuck time, per-role kill credit; fixtures replay with stable hashes and the existing audience observer reads real events.

## Cross-stage regression suite

Implement/fix specific named fixtures from [the design spec](../03-tech/companion-ai-command-system.md#12-representative-tests-headless-first): C-01 through C-16. Keep existing `solo_swap_continuity`, `solo_follower_dodge`, `solo_guard_floors_at_one`, `duo_independent_moves`, Donut mana/unlock, boss and E1 gates fixtures intact. Extend them only where the real simulator supports a scenario. Do not replace the simulator, regenerate old goldens blindly or mark a docs-only design proposal as a tested feature.

Baseline commands when relevant:

```sh
python3 scripts/check_docs.py
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
bash scripts/ci_floor01_combat.sh headless
```

Use actual, currently supported script flags. Supply replay hashes, ordered event traces, real rendering captures for visual changes and CPU cost for AI planning. If a tool or hardware device is missing, report the exact blocker; **do not fabricate check results**.

## PR description and review handoff

Each stage PR includes: **Why**, **Change**, **Acceptance Criteria** (checked only when verified), **Commands & Results**, **Visual Evidence**, **Performance Data**, **Rights/Provenance**, **Risks**, **Next PR**. An independent gameplay/architecture reviewer checks the final head; subsequent merged mechanics stay **game-feel review pending** until the owner plays. The coordinator updates the backlog status per stage; do not silently mark all COMPANION-AI-01 work complete after 01A.

**First agent task:** Implement 01A only: deterministic safe *combat support* for either AI Carl or AI Donut through real party physics, tests and traces. Leave controller commands and HUD untouched; the other agents can tackle the isolated 01B/01C spikes in parallel.
