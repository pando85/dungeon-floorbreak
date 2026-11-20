# COMBAT-LAB-01 — Local agent implementation work order

**Task:** implement the approved advanced mid-progression combat sandbox and test course, **not** another opening tutorial.
**Authority:** [combat-lab design](../01-design/combat-lab-01-advanced-playtest.md), [PLAYTRACE-01 recorder](../03-tech/playtrace-01-session-capture.md), [playtest protocol](../04-production/combat-lab-01-playtest-protocol.md), [character progression](../01-design/character-progression-horizon.md).
**Status:** implementation requested, **nothing in this design PR is a playable level**.

## Required audit before starting

Read `AGENTS.md`, `docs/07-agents/workorders.md`, `docs/07-agents/agent-orchestration.md`, `docs/01-design/combat-first-encounter-design.md`, COMBAT-FUN-02 spec/work order, companion AI spec/work order, AUTOPLAY-01 + AUTOPLAY-02a, `src/party_scene.rs`, `src/combat_scene.rs`, `src/corridor.rs`, current Bevy level layout/input/camera/asset pipelines, approved enemy art roster and ENV-01/02. Examine open PRs before assigning shared files. Verify available build features and log missing capabilities: never ship a placeholder monster as if its visual concept included approved combat behavior.

## PR plan and acceptance evidence

| Stage | Owns | Deliverable / tests | Dependencies |
| --- | --- | --- | --- |
| LAB-00 | New lab scenario/config manifests, validator, harness/room selection | Room IDs R00–R06, stable seed, player mode, capability profile, feature gates, test-only launch and room restart; parser rejects impossible/unimplemented abilities, unknown enemy/move IDs and unresolvable camera/geometries | Core party simulation; recorder PT-A contract in parallel |
| LAB-01 | Geometry for R00–R02, existing enemies, camera fixtures | Quality solid platforms, readable vertical spaces/cover, combat tests for melee/body hits, small ledges, llama evasion and both-character reachability; 640×360/400/1080p clips in original art style; fixed room seed | CF02 collision/air/F changes as integrated; PT-B before first new human test |
| LAB-02 | R03 and R05 only; destructible level data, authored wave orchestration | Support/bridge mechanics, protected return route, multiple ways of killing, 3–5 concurrently active default, deterministic reset and performance; R05 supports three distinct practical routes | LAB-01; authoritative terrain/hazard events; cannot invent new physics |
| LAB-03 | R04, role-control + coop integration tests | Carl-human/Donut-AI, Donut-human/Carl-AI, P1+P2: same path/no softlocks; camera, input ownership, mana, moves and commands; R04 tests ranged+melee pressure with **existing** goblins+llama first | LAB-02, COMPANION-AI behavior/navigation/command integration and party-mode test readiness |
| LAB-04 | New-archetype interfaces and isolated single-enemy probes | Add Rat, Bomb Bard, Troglodyte or Scatterer **only after** original behavior/telegraph/animation proposals and design review; one-at-a-time A/B proof of distinct tactical role and no art duplication | Separate enemy design and production implementation PRs; optional for first complete lab |
| LAB-05 | Playtest evidence, acceptance/iteration | Owner test (including real two pads), fixed-seed A/B runs, repeat arena three times, ratings, camera traces, ML-ready recordings, replay reports, honest tuning decisions | LAB-01–03, PT-B/C/D, optional LAB-04 |

**Avoid large conflicting gameplay PRs.** Agents can work on environment/level data and recording in parallel without both editing `src/combat_scene.rs` or `src/corridor.rs`. The integration owner makes the minimal common adapter after upstream rule changes land. Do not copy a `cf02_microarena` implementation or own COMBAT-FUN-02 mechanics in this workstream.

### LAB-00 exact behaviors

Implement only the smallest possible **real** scenario descriptor using current LDtk/JSON/fixture conventions. For each room record `id`, layout/config/content hash, seed/variant, difficulty preset, allowed hero modes, legal capability profile, encounter/wave IDs, hazard/terrain IDs, checkpoints, camera/visibility constraints, goal and recording segment boundaries. The loader validates reachability using implemented jump/physics as far as existing tools permit, otherwise emits explicit `validation_pending` and requires an integration fixture. Multiple rooms are one user session but separate measurable episodes/encounters. Direct room entry/retry and fixed-seed A/B must work without campaign saves.

### LAB-01/02 specific geometry quality gates

Both characters must be able to reach relevant shared ground progression; Donut should gain at least one **optional offensive** elevated route Carl cannot immediately take from the same starting point, not a mandatory mirrored path. No jump in the main route should be an unexplained blind leap. Build visibility and traversal probes: before destruction, after partial destruction, after death/checkpoint, after swap, with 2P separation, under native 640×360 and 640×400 cameras, in large-window scaling and at 1× speed. Ensure goblins that cannot climb cannot be required to do so. Llama pressure must have at least two visibly learnable responses. No untelegraphed offscreen falling loads. Cover/perches may be solid; one-way/drop-through requires its own approved movement contract.

### LAB-03 companion and co-op gates

Log if AI helps/steals kills, saves a character, becomes stuck, interrupts a duo combo, or cannot reach a mandatory gate. Demonstrate with objective telemetry and actual play: human Donut and AI Carl works as well as the opposite assignment; player can choose to switch; all contextual orders have honest acceptance/refusal; no AI-only teleport/unearned mana. Two-pad proof must use the true `PartyState` path, not two Carl-only simulations.

### LAB-04 new-enemy rule

Each candidate has a small design sheet with player-readable tell, role, legal attacks/resources, size/collider, counterplay, 1P/2P/AI interaction, death/impact type, environmental interaction, animation source provenance, reproduction fixture and independent review. No "fully implemented" claim from an approved concept PNG. No blanket new equipment, bullet-hell or health sponges. First ship with goblins and llama, add candidate monsters as optional feature-gated encounters.

## Hard no-regression criteria

- Legacy DEMO-01, cf02-off game rules, scene state hashes and pinned AUTOPLAY `autoplay-episode/1` goldens remain unchanged.
- Recording is **automatic in development/test launches** once PT-B exists; every gameplay attempt and retry, including tests outside LAB, gets an explicit recording status. No full owner playtest is labelled `ML-ready` without PT-C/D exporter + verification.
- Gameplay outcomes and performance do not change when capture is enabled/disabled on identical fixed-tick inputs.
- No artificial reward/drop/boon/viewer availability in the campaign, regardless of lab experimental loadout.
- Existing controller mapping remains the same. All timings and enemy/geometry counts are tunable hypotheses, not signed-off balance.
- All tests and reported build commands actually run. Attach short at-speed clips for gameplay, verified session manifest/replay digest for recording, perf before/after and reports using the [playtest protocol](../04-production/combat-lab-01-playtest-protocol.md).

## Required review

Follow `AGENTS.md` agent model tiers and independent review, plus owner game-feel signoff after code merge. The design and implementation PRs must state clearly what is implemented, still experimental, untested on hardware, or not ML-eligible. All rights and art source rules still apply.
