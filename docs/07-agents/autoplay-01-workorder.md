# AUTOPLAY-01 — local agent implementation work order

**Work item:** AUTOPLAY-01 (new). **Design PR:** AUTOPLAY-01 headless gameplay evaluation. **Authoritative spec:** [AUTOPLAY-01 technical specification](../03-tech/autoplay-01-automated-gameplay-evaluation.md). **Status:** implementation requested after this design PR; nothing here claims an implemented trainer or measured speed. **Adopted defaults:** every open choice is closed by an `AP-Q##` entry in [spec section 10](../03-tech/autoplay-01-automated-gameplay-evaluation.md#10-adopted-defaults-ap-q); cite the IDs you implement in each PR. The owner may change any of them in a follow-up PR.

## Mission

Deliver a **real, locally runnable automated gameplay lab** on top of the repository's existing deterministic Rust combat simulation. It must be easy to evaluate scripted agents, train a small PPO policy via Gymnasium, compare difficulty/strategy diagnostics across game commits and replay interesting episodes through the same authoritative game rules.

**Do not build a second toy platformer.** The acceptance proof must exercise `FixtureSim` / `CombatState` / `SceneSim` and the existing `RawInput` press-edge semantics. Bots and the ML environment drive **the game**, not a simplified reimplementation.

## Read order and status audit

1. `AGENTS.md`, `docs/07-agents/workorders.md`, `docs/07-agents/agent-orchestration.md`, and the [AUTOPLAY-01 spec](../03-tech/autoplay-01-automated-gameplay-evaluation.md).
2. `src/combat_scene.rs` (`Fixture`, `FixtureSim`, `CombatState::step`/`step_party`, `take_new_events` and `state_hash`), `src/corridor.rs`, `src/input.rs`, `src/party.rs`, `src/party_scene.rs` and `src/demo.rs`.
3. `scripts/ci_floor01_combat.sh` and existing fixtures/golden replays under `docs/evidence`; [testing documentation](../03-tech/testing.md) and [architecture](../03-tech/architecture.md).
4. `docs/01-design/mvp-controller-contract.md`, `combat-first-encounter-design.md`, `character-progression-horizon.md`, and the active [COMBAT-FUN-02 proposal PR #124](https://github.com/pando85/dungeon-floorbreak/pull/124) for baseline context only. PR #124 is **not a merge dependency**.
5. Check current main and open gameplay PRs before writing any shared file. Reconciled against main `46210b1d`: Donut Q10/mana (#123), CINE-01 (#122), AUDIENCE-01 (#118) and THE JUICER boss (#120) are merged and all excluded or bypassed in v1 (spec section 1). COMBAT-FUN-02 (#124) stays behind `cf02_rules` (AP-Q05, AP-Q11, AP-Q12).

## Delivery sequence

Implement the stages as **four separate small PRs in order A, B, C, D** (AP-Q21); each lists the shared files it touches. Preserve one source of truth and report each stage's completion independently. Any unfinished stage must stay explicitly unchecked.

### A. Pure Rust playable environment (first mergeable unit)

- Files: `src/autoplay/{mod,action,observation,episode,reward,scenario}.rs`, `assets/data/autoplay/reward_profiles_v1.json`, and one `pub mod autoplay;` line in `src/lib.rs` (the only shared file; AP-Q01, AP-Q15). No new Cargo dependency, no edit to any CF02 collision file.
- Write a small Bevy-free `AutoplayEnv` wrapping the current fixture/scene/combat state. Load validated existing `EnvKit`/`Fixture` combinations using the normal asset root; expose `reset(seed)`, `step(ActionV1)`, `observe()` and immutable episode diagnostics; use exactly the same 60Hz combat rules.
- Use the **Carl-only, single-fight** v1 roster of AP-Q17 (`cp01_single_goblin`, `ap_two_goblins_wide` for development; `cp02_two_goblins`, `ap_single_goblin_far` held out); `cp02_kick_contact` (Passive goblins) is a parity probe only (AP-Q18). Do not use a generic scenario runner presumed to exist; the repo's existing scripted fixture runner is real.
- Implement the action mapping as the spec's `MultiDiscrete([3,2,2,2,2,2])` with real held-button edges, reserved indices 4/5 and strict validation (AP-Q03, AP-Q04; adopt `RawInput::NONE` when CF02-00 adds it). `action_repeat` per AP-Q16. Config carries `action_version`, `cf02_rules` (reject `true` until CF02-00) and `sim_seed` (reject non-default until CF02-G) per AP-Q05, AP-Q11, AP-Q12.
- Terminal and truncation per AP-Q09/AP-Q10: stop on the defeating step (the fixture restarts 49 ticks later), defeat > success > abandoned, `exit_east` is `abandoned`, tick counter is the env's own executed-tick count. Reset constructs a **fresh** `FixtureSim` and verifies initial hash parity (AP-Q11); never call `FixtureSim::restart`. Unsupported fixture/layout/overflow throws, not silently simplified physics.
- Implement ObservationV1 exactly as AP-Q06 with the AP-Q07 perception mask; `info` vs diagnostics per AP-Q08, including the differential leakage test (health pairs on the same side of max, `damaged` as the one intentional health bit), feature clamping with the `obs_clipped` counter and the foot-line `layout_y` definition of `y_in_view`. Isolation tests per AP-Q13. Roster tests per AP-Q17: initial `enemy_mask` per scenario, spawns at least 16 px from the initial view edge, and no `ap_*` name collision with `Fixture::by_name` or `FIXTURES`.
- **Acceptance A:** the same per-tick input sequence produces the same `state_hash`, gameplay events and outcome through the native harness as through the existing fixture path (AP-Q18 parity: ungated `step_raw` over `fixture.ticks` ticks against a live `FixtureSim` reference and copied literal golden hashes for `cp01_single_goblin`, `cp02_two_goblins` and `cp02_kick_contact`, never referencing the private `EXPECTED_HASHES`); repeated reset/seed parity and terminal states have unit tests.

### B. Native evaluation, baselines and reliable evidence

- Files: `src/autoplay/{bots,metrics,replay}.rs`, `src/bin/autoplay.rs`, `docs/evidence/autoplay-01/` (no shared file). Episode format AP-Q19, attribution AP-Q14; bots see only `ObservationV1` (AP-Q08).
- Add a small CLI: enumerate supported scenarios, run one bot, evaluate a batch, replay/verify a stored episode, and benchmark release headless ticks/s. Use stable command conventions; document exact commands that work.
- Ship three reproducible baselines (`random_legal`, `approach_and_attack`, `reactive_survivor`), and an **event-driven** telemetry collector. Do not have heuristics read private timers, future scripted inputs, hidden foes or privileged `info`.
- Record versioned metadata, per-tick normalized action traces, ordered authoritative events, sampled/final game state hashes and actual terminal reason. Use seeded reproducibility; don't pretend a seed changes static fixture physics.
- Produce machine-readable per-episode + aggregate metrics (JSON and CSV), explicit success confidence interval, times/damage, action usage, enemy contact/death causes, no-progress, unreachable/softlock diagnostics where observable and replay pointers. If source events don't carry attribution, record `unknown` instead of guessing.
- Split truly different **scenario configurations** into development and hold-out evaluation; prevent exact duplicates masquerading as generalization. Pin both scenario roster and bot settings so an agent can compare two different game commits fairly.
- **Acceptance B:** a replay generated by a bot verifies identically in a fresh Rust process (same tick/event hashes); one intentionally corrupted action fails replay verification; one fixture overflow fails clearly; summary displays number of episodes and which cases were held out.

### C. Optional Python dependency bridge and a first real ML training run

- Files: `tools/autoplay/**` (standalone crate with own `[workspace]` and `Cargo.lock`, `pyproject.toml`, `uv.lock`, package, tests), new `.github/workflows/autoplay-tools.yml`, `.gitignore` entries (AP-Q02, AP-Q20). Pinned versions and the uv-managed Python 3.12 venv are in AP-Q20.
- Build a separate tooling-only native binding with PyO3/maturin (preferred). Do not put Python/torch/Gymnasium into normal game Cargo or its runtime. Add a small Gymnasium adapter that conforms to the correct `reset`/`step` five-tuple, fixed action/observation spaces and `check_env`.
- Provide a reproducible local installation recipe and a smoke script: *build Rust binding -> run Python env checker -> evaluate a baseline -> verify its native replay -> train a small PPO model -> reload and evaluate*. All steps should run CPU-only without external accounts, services or assets.
- Use Stable-Baselines3 `PPO("MultiInputPolicy", env)` with bounded, configurable training steps. A **10,000 timestep smoke** is enough to prove training/save/load; it does not have to beat a baseline or succeed consistently. Seed Python/NumPy/PyTorch, environment and action sampling; persist version/config metadata alongside checkpoints.
- Test installed package behavior from outside the source tree to avoid false-positive imports, and document/measure extension/step overhead. Add batching only if justified by measurement.
- **Acceptance C:** a clean local setup can run `gymnasium.utils.env_checker.check_env`, at least one deterministic evaluation set, a bounded PPO training job and reloaded model evaluation; no training model weights or bulky traces enter git.

### D. Final integration and reproducible benchmark report

- Verify existing headless combat, demo, party and boss goldens are unchanged (or explicitly explain and migrate a *genuine* required change; default expectation is **no gameplay change**).
- Measure Rust-only and Python single-env simulated ticks/s; if it is slower than real-time, identify a bottleneck rather than promising a speedup. Record toolchain, OS, CPU, build profile, fixture/observation complexity and repeat count. Frame render timing is not training throughput.
- Files: `docs/evidence/autoplay-01/README.md`, `docs/04-production/backlog.md`, one line in `docs/03-tech/testing.md`.
- Run all appropriate project format, lint, unit, docs and Python tests; the isolated opt-in Python job of AP-Q20 (no torch) must be green; PPO stays local.
- Evidence under `docs/evidence/autoplay-01/README.md`: command transcripts, version info, demo evaluation report, 1 or 2 short replay checks, training smoke output, honest performance table, and known unsupported cases.
- **Acceptance D:** one final concise report with reproducible commands, commit SHA, measured metrics, model reproducibility metadata, test status, limitations and intended follow-up. A human can reproduce the test locally without a special agent platform.

## Minimal public interface and CLI behavior

Expected native API semantics:

```text
env = AutoplayEnv(config, assets_root)
obs, reset_info = env.reset(seed)
next_obs, reward, terminated, truncated, info = env.step(action)
# Cannot call step again until reset after terminated/truncated.
```

```text
cargo run --release --bin autoplay -- list-scenarios
cargo run --release --bin autoplay -- evaluate --scenario cp01_single_goblin --policy reactive_survivor --episodes 50 --out artifacts/autoplay-01/eval
cargo run --release --bin autoplay -- replay --episode artifacts/autoplay-01/eval/episode_0001.jsonl --verify
cargo run --release --bin autoplay -- benchmark --scenario cp01_single_goblin --episodes 1000
```

```text
# After isolated Python tooling setup:
python -m autoplay evaluate --scenario cp01_single_goblin --policy reactive_survivor --episodes 50 --out artifacts/autoplay-01/python-eval
# replays land in <out>/replays/<scenario>/<policy>/episode_0001.jsonl; verify with the native CLI:
# autoplay replay --episode artifacts/autoplay-01/python-eval/replays/cp01_single_goblin/reactive_survivor/episode_0001.jsonl --verify
python -m autoplay train --algorithm ppo --scenario cp01_single_goblin --steps 10000 --seed 42 --out artifacts/autoplay-01/ppo-smoke
python -m autoplay evaluate --scenario cp02_two_goblins --checkpoint artifacts/autoplay-01/ppo-smoke --episodes 50 --out artifacts/autoplay-01/held-out
```

These are **interfaces to implement**, not commands available today. If actual packaging requires different commands, update both documents and tests with the verified alternatives. Avoid fake evidence or claiming a static seed is domain randomization.

## Non-negotiable test cases

| Test | Expected result |
| --- | --- |
| Same initial fixture and reset twice | Same starting hash, observation, event cursor and allowed actions |
| Same actions + same scenario from scratch | Identical authoritative ordered events, hashes and terminal outcome |
| Hold attack during multiple steps | One press edge until released and pressed again |
| Simultaneous buttons | Existing Rust action precedence used; no invented Python combat order |
| Detect defeat before the automatic restart | One terminated episode on the defeating step, no restart inside it (AP-Q09) |
| Timeout while alive | Truncated, not defeated; final observation retained |
| Reward from one defeated foe | Credited once; no bonus from repeated blood/GIF/audio frames |
| Hidden enemy / hidden trigger | Absent from policy observation; optionally visible in separate diagnostics |
| Locked special / swap | Forwarded as the sim's real no-op, `available = 0`, unavailable presses and held ticks counted (AP-Q03) |
| Same-tick clear and defeat | Outcome `defeat`, diagnostic `simultaneous_clear` (AP-Q09) |
| Exit reached with foes alive | Terminated `abandoned`, not success (AP-Q09) |
| Incompatible fixture/replay versions | Explicit rejection/migration needed |
| Native replay after Python/agent run | Same game event + tick hashes |
| Independent parallel envs | No cross-env mutable/global state; interleaved and threaded runs equal sequential runs (AP-Q13) |
| `cf02_rules` or `sim_seed` mismatch | Config or replay rejected with a clear error (AP-Q11, AP-Q12) |
| Existing fixture, demo, party, boss | Their goldens and state/event semantics remain stable |
| PPO save/reload | Loads checkpoint with matching schema; evaluates without training state |
| Bounded stress run | Memory and per-step runtime reported honestly; no file explosion |

## Out-of-scope now (write proposals, not code)

- PettingZoo and synchronized 2P/companion-vs-enemy multi-agent learning (**AUTOPLAY-02**).
- Shipping a neural policy inside Bevy or replacing the authored goblin/Donut IA (**AUTOPLAY-03**).
- Automated level generation, parameter optimization or automatic balance changes (**AUTOPLAY-04**).
- Visual pixel observations, online integrations, account data capture, video training, new art, new enemies, new combat moves, CINE/audience scoring changes.

**Important:** player-trained agents are **testing instruments**, not automatically believable humans. Calibrating difficulty against players must use voluntary real playtest traces; avoiding unbeatable enemy AI and passive/overpowered Donut requires real human judgement. The current COMBAT-FUN-02 evaluation continues independently.

## Proposed handoff division if multiple local agents are used

1. **Integrator / Rust (medium tier):** native core, schema, scenarios, determinism and replay.
2. **Python tooling (medium tier):** adapter, bots, evaluation report and PPO smoke **after Rust v1 schema is frozen**.
3. **QA (medium tier):** independent parity/cross-process tests, benchmark, CI reproducibility.
4. **Independent reviewer (top tier):** one focused architecture/design review of observation leakage (AP-Q06 to AP-Q08), terminal semantics (AP-Q09, AP-Q10) and event attribution (AP-Q14); then standard code review and normal AGENTS.md merge rules.

No writes to `src/combat_scene.rs`, `src/corridor.rs`, `src/input.rs`, `src/party.rs` or any other COMBAT-FUN-02 file; the only shared edits are those listed per stage (AP-Q21). The CF02 seed-override hook (AP-Q11) belongs to the CF02 lane. Prefer additive Rust modules and a separate optional tooling package. If the existing game API needs a change, isolate it in the smallest possible PR and coordinate its merge with the combat integrator.

## Agent report contract

Report: implemented units A/B/C/D, exact commands and results, final head SHA, stable schema versions, Rust and Python throughput figures, files/evidence links, any test not run, known cheating/observation limits, and next stage. Use the repo's PR body template (Why, Changes, Acceptance, Testing, Evidence, Performance, Rights, Risks, Next PR). **Do not label game balance, human difficulty or game fun "validated" by automated agents alone.**
