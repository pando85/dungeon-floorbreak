# CORRAL-AUTO-01 — Autonomous regression and exploit-search protocol for The Infernal Corral

**Owner request (2026-11-21):** the local agent MUST independently **implement, run, interpret, iterate and report** automated gameplay tests for this encounter. Merely writing the tests, mentioning AUTOPLAY, or checking that the level loads is **not complete**. This is an *implementation obligation* attached to [PR #181](https://github.com/pando85/dungeon-floorbreak/pull/181), which at the time of this document is still specification-only.

**Authoritative scene design:** [The Infernal Corral v0.2](../01-design/floor-01-llama-magic-baptism.md). **Main objective:** demonstrate that the M0/M1 spell empowerment works, but the **M2 corral cannot be reliably trivialized by a stationary Donut repeatedly casting**. This is a hard gameplay design regression, not only a test that some enemy reaches a timer.

## 1. What existing automation actually supports (checked 2026-11-21)

- `src/autoplay/party/env.rs` implements a real `PartyFixtureSim`-backed **`party_solo` environment**, with the live party step order. `src/autoplay/party/scenario.rs` currently registers **goblin-only `ap2a_*` scenarios**. It does **not** expose the proposed llama corral as a scenario.
- `src/bin/autoplay.rs` supports `autoplay list-scenarios --mode party_solo` and `autoplay run --mode party_solo --seat donut|carl --scenario ... --policy ...`. Its **party evaluate, party replay and output-file support are pending AUTOPLAY-02a-C**; do not pretend `autoplay evaluate --mode party_solo` exists or assume a generic ML agent can test this encounter without an adapter.
- The normal AUTOPLAY-01 `evaluate`/`replay` infrastructure exists **for Carl-only v1**, and Playtrace records live dev/manual sessions and has verification support; it is not necessarily a drop-in multi-agent corral benchmark.
- The reusable *source of truth* for the diagnostic is the **real Rust `PartyFixtureSim` / `CombatState` / `SceneSim` / `SlotInputs`** with the actual future M2 scene, enemy states, mana, collision, level geometry and progression flags. Do **not** implement a simplified Python/RL toy with imitated lava/rush behavior.

### Mandatory delivery even if AUTOPLAY-02a-C is still pending

Add a **small Rust headless test/evaluator** (prefer a dedicated scenario/test module and an optional executable runner rather than invasive general CLI rewrites) that drives the real party scene. Only extend the `party_solo` AUTOPLAY roster/CLI if its interfaces are compatible and existing maintainers agree. The tests must run in a standard local Rust build with **no Python, GPU or trained policy required**.

The agent must inspect the current branch for merged 02a-C, companion and character-progression updates just before implementation; adapt to real APIs and document exact **working** commands, without silently assuming currently unavailable flags.

## 2. Mandatory policies (same actual input channel as humans)

| Policy ID | Action policy | Why |
| --- | --- | --- |
| `stationary_cast` | Donut starts at a **legal, authored x/y and facing**, holds **zero horizontal movement** and never jumps, pounces or swaps. Generate a new *Primary press edge* whenever a legal cast can begin and mana permits. Continue through regen. | Exploit we are explicitly trying to rule out |
| `stationary_cast_burst` | Same position and actions, but send valid closely spaced 8-charge burst, then wait for refill. No invented mana bypass. | Strongest stationary DPS schedule; avoids missing a different optimal cadence |
| `mobile_cast` | Donut moves between clear real floor/ledge positions to dodge telegraphed rush or gain LoS and shoots at viable targets. The bot uses **observable current-state signals**, not future frames or private decisions. | Counterfactual: is deliberate movement actually useful? |
| `carl_melee` | Carl approaches under existing movement, jumps/uses cover if needed and punishes a real committed rush or spit recovery with valid punch/kick inputs. | Ensure melee is not made irrelevant by the anti-spam fix |
| `coop_probe` | Drive P1 Carl + P2 Donut with two actual `SlotInputs` at the same 60Hz tick, with controlled spacing/coordination alternatives. | Check that optional team play is viable, not two separate independent simulations |
| `no_interaction` | Neutral inputs at multiple locations, to observe enemy self-stall, approach and threat readability. | Diagnose whether the new llama AI actually challenges stationary positions |

**Press-edge semantics are non-negotiable:** sending `attack=true` for 120 ticks produces one press, not 120 spells. The evaluator must issue correct press→release transitions, including the 16-tick cast/refire boundary, real release tick 4, 8-charge mana pool, 30-tick regen delay and 40-tick recharge period. Do not call `missile_hits_llama()`, mutate `mana`, set enemy health or trigger a rush from the evaluator: all outcomes must result from real per-tick input and simulation.

The policies should be fixed, versioned configurations and **not** auto-switch to a more capable policy mid-run merely to turn a loss into a win. A source decision to activate a real preexisting companion should be reported; diagnostic isolation should keep the companion's contributions measurable.

## 3. Sweep across actual player starts

Generate **valid player-start variations through the existing fixture/scene authoring layer** (not teleporting a hero during combat):

1. **M1 single-llama shot line**: the first power fantasy may yield a fast, safe stationary magic win. **Positive control**.
2. **M2 natural entry**: where a novice stops when threats become visible.
3. **M2 approach lane**: nearest legal ground position from which Magic Missile can hit.
4. **M2 just behind/at an edge of real cover**: including LoS obstruction and an escape route.
5. **M2 best apparent safe ground angle** discovered by a simple deterministic grid sweep.
6. **M2 +32px ledge and optional alternate ledge** if built: projectile height/rush geometry must be authentic.
7. **M2 near the vulnerable partition**: with partition intact and, in a separate authored test case, legitimately broken.
8. **M2 both target directions and valid player-control modes**. Test Carl/Donut target selection rather than always pointing a llama at Carl.

**No false RNG evidence:** the present base fixtures can be deterministic with a seed that does **not** influence gameplay; running 50 seeds through one identical config is not 50 independent encounters. If the rules contain no meaningful seeded variation, **enumerate actual locations, facing, attack phase or initial room configurations** and repeat exact input only for determinism confirmation. Record sample independence and fixture IDs honestly.

**Test horizon:** start with up to **1,800 fixed ticks (30 seconds)** per isolated M2 attempt, configurable, and also test the complete M0→M3 route with a larger timeout appropriate to level length. Do not call an attempt successful just because `tick_limit` elapsed or because one llama died. Required M2 victory condition is **both enemies actually defeated** (or the explicit real authored stage-clearing rule, if different).

## 4. Required numeric metrics (save even for failures)

For every variant/policy/attempt, capture:

- `run_id`, build/commit SHA, ruleset/profile, fixture/room/content hashes, base capability flags, human seat/AI companion policy, run setup and action-policy version.
- Actual initial `x/y/facing`, initial/max/final HP and mana, time ticks, physical distance traveled by **the tested hero**, actions pressed and bolts **actually released** (not held presses), shots hit/missed.
- For each llama: initially visible/active, final HP, target-selection source, `spit_tell`, glob release, `rush_tell`, `rush_commit`, `rush_contact`, `rush_miss`, recovery, interrupted tells, final defeat.
- Damage by hero vs companion, magic vs melee vs environmental source; player damage and whether a particular attack was evaded. Use `unknown` if current event schema cannot attribute honestly; do **not** fake a source.
- Current cover/partition state at start/end, real line-of-sight changes, used positions/jumps, genuine collision with partition or miss.
- Terminal reason **`victory | hero_defeat | timeout | softlock | invalid_setup`** (or stable equivalent), scenario and policy version, final authoritative state hash, trace/replay pointer, plus diagnostics for blocked capture.

**Derived flags:** `stationary_position_valid`, `no_damage_clear`, `no_movement_clear`, `both_llamas_defeated`, `attempts_with_targeted_rush`, `movement_displacement_px`, `companion_kill_contribution`. Differentiate **Donut solo behavior** from **Donut + active Carl AI**; never report a companion-carried win as a pure stationary-Magic-Missile victory.

## 5. Gate logic (agent must evaluate, not merely log)

**Positive control:** M1 permits/should showcase a straightforward Donut 3-bolt win. If Donut's projectile cannot physically hit a llama in the story rules, **fail** and fix the ability integration (do not skip or switch to a fake damage path).

**Hard M2 failure:** if **any naturally reachable, reproducibly selected static position** clears both llamas at negligible player risk using only legal stationary missile presses, **fail this scenario design**. Try a few independent legal positions and reveal the **best counterexample**, not just a random average win rate. Don't mask one real degenerate spot with a sea of impossible bad positions.

**Movement success:** moving Donut should have a demonstrable, mechanically legitimate way to beat both llamas at normal difficulty; Carl-only and 2P must also have viable direct plans. If all policies lose, or a rush hits before its tell, **fail**: preventing stationary wins by making the game unplayable is not a valid fix.

**No shortcut by balancing cheats:** do not "fix" the gate by increasing enemy HP, applying magic resistance, arbitrarily reducing missile range/regen, assigning global immunity, introducing untelegraphed homing glob or always killing stationary actors on a timer. Improve **real approach/charge logic, stage geometry and authored LoS** while preserving an honest first magic win.

**Fairness invariant:** a player who sees and responds to the charge must have a feasible safe movement/dodge route with real speed and collision. A stationary player *may* sometimes survive a charge with lucky enemy positioning, but this must not be a universally unpunished stationary-DPS solution.

**Hard system failures:** a test detects a frozen/stuck enemy with no reachable exit, hero/llama moving outside bounds, missing actual target selection (Carl-only), invalid party ownership, non-deterministic same-input results or failure to reset an attack/cover state on retry.

**Report must distinguish** `pass`, `fail`, `blocked` and `not_run`. A `blocked` test is not a `pass`. If one of the hard gates fails, mark the implementation PR **not ready** and report the concrete smallest counterexample.

## 6. Executable integration expectations

Minimum commit set by the local agent:

1. A deterministic **fixture/authoring entry** for `infernal_corral_m1` and `infernal_corral_m2` in the **real** scene/party path. The first isolates the allowed easy win; the second has two live llamas and the real rush/cover states.
2. A headless Rust **automated test module** for stationary exploit, moving win, Carl melee, 2P, attack tells, replay hashes, retry and physical partition impact. Tests must **assert outcomes**; a test that only prints is insufficient.
3. A small evaluator or test option that runs all declared positions/policies and emits machine-readable **JSON summary + CSV run rows** (and a short Markdown report generated from that data). If easy, integrate with future AUTOPLAY-02a-C; otherwise keep it as a scoped local binary/test helper built around `PartyFixtureSim`.
4. Explicit **commands** to run the test suite and evaluation on a fresh checkout. Suggested convention only, to be provided after authoring the actual tests:

   ```bash
   cargo test --locked corral_auto -- --nocapture
   cargo run --release --locked --bin corral_eval -- --out target/corral-eval
   ```

   `corral_eval` and `corral_auto` are **deliverable names to implement**, not commands that already exist. The agent must not tell the owner to run an unimplemented CLI.
5. **Two identical replays** of at least one static-exploit counterexample (if found) and one mobile success, with identical event ordering and state hashes under the same config/inputs. Use current `playtrace verify` for true live sessions when available; when party-autoplay episode replay is not yet implemented, provide a deterministic **Rust input/event/hash replay assertion** with the existing sim rather than inventing a CLI feature.
6. Auto-evaluation **before** tuning, **after each meaningful fix**, and on the final branch head. Emit a clear **before/after differential** identifying what changed and whether the stationary loophole was closed *without* making ordinary combat unfair.

**Suggested generated evidence paths:** `target/corral-eval/<run_id>/summary.json`, `runs.csv`, `traces/...`, and `report.md`. Commit a concise human-readable summary or small pinned fixture evidence under `docs/evidence/floor01-corral/` if consistent with repo conventions; do not commit gigantic recordings, training weights or private player data.

## 7. The agent's required autonomous loop

```text
Audit and construct true M1 / M2 fixtures
          ↓
Implement baseline stationary and mobile action policies
          ↓
Run location/attack-phase sweep against the actual sim
          ↓
Read summary + inspect lowest-risk stationary success traces
          ↓
If M2 has safe static clear: FIX enemy movement / telegraph / geometry
          ↳ rerun sweep; preserve first llama empowerment and honest controls
          ↓
If all bots lose: validate actual movement routes and ordinary direct fighting
          ↳ simplify unfair geometry or attack timing; rerun
          ↓
Check solo Carl, solo Donut, P1+P2 and both side/height/partition states
          ↓
Verify same-seed, same-input event/hash determinism + checkpoint reset
          ↓
Attach machine results, failures, and real Bevy playtest evidence
          ↓
Mark machine gates pass or honestly blocked; human fun review remains pending
```

The agent is **authorized to tune the new corral enemy rush and room geometry within the existing design constraints**, then re-evaluate. **It is NOT authorized to rewrite Donut movement, 12s reflex, generic companion AI, spell economy, existing non-corral llama fixtures or shared Playtrace schemas** merely to make the test pass.

## 8. Explicit acceptance checklist for agent PR

- [ ] Actual **M1 static success** and no false mana/shot accounting
- [ ] M2 **best natural stationary exploit** search; no reproducible low-risk win by stationary Donut with legal input
- [ ] M2 **mobile Donut success**, normal Carl melee route and genuine local 2P success
- [ ] The rush is real, telegraphed, committed, cancelable/interruptible only by actual combat rules, with a punishable miss
- [ ] The optional partition is truly collider-driven and cannot softlock the stage
- [ ] Each test reports actual llama targeting and contribution by companion vs active player
- [ ] Deterministic fixed-tick restart and replay hash/event equality
- [ ] Regressions to current combat, mana and camera suites remain green
- [ ] Correct JSON/CSV output, commands tested by the agent, **evidence attached to PR**
- [ ] Real human at-speed Bevy playtest and owner game-feel review stated separately; never claim machine tests certify fun

**If automation infrastructure or the corral gameplay is still absent, report those as explicit prerequisite implementation work**, not as a passing test. This document is a contract for the implementing agent, not evidence the evaluator has already run.
