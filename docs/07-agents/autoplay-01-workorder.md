# AUTOPLAY-01 — local agent implementation work order

**Work item:** AUTOPLAY-01 (new). **Design PR:** AUTOPLAY-01 headless gameplay evaluation. **Authoritative spec:** [AUTOPLAY-01 technical specification](../03-tech/autoplay-01-automated-gameplay-evaluation.md). **Status:** implementation requested after this design PR; nothing here claims an implemented trainer or measured speed.

## Mission

Deliver a **real, locally runnable automated gameplay lab** on top of the repository's existing deterministic Rust combat simulation. It must be easy to evaluate scripted agents, train a small PPO policy via Gymnasium, compare difficulty/strategy diagnostics across game commits and replay interesting episodes through the same authoritative game rules.

**Do not build a second toy platformer.** The acceptance proof must exercise `FixtureSim` / `CombatState` / `SceneSim` and the existing `RawInput` press-edge semantics. Bots and the ML environment drive **the game**, not a simplified reimplementation.

## Read order and status audit

1. `AGENTS.md`, `docs/07-agents/workorders.md`, `docs/07-agents/agent-orchestration.md`, and the [AUTOPLAY-01 spec](../03-tech/autoplay-01-automated-gameplay-evaluation.md).
2. `src/combat_scene.rs` (`Fixture`, `FixtureSim`, `CombatState::step`/`step_party`, `take_new_events` and `state_hash`), `src/corridor.rs`, `src/input.rs`, `src/party.rs`, `src/party_scene.rs` and `src/demo.rs`.
3. `scripts/ci_floor01_combat.sh` and existing fixtures/golden replays under `docs/evidence`; [testing documentation](../03-tech/testing.md) and [architecture](../03-tech/architecture.md).
4. `docs/01-design/mvp-controller-contract.md`, `combat-first-encounter-design.md`, `character-progression-horizon.md`, and the active [COMBAT-FUN-02 proposal PR #124](https://github.com/pando85/dungeon-floorbreak/pull/124) for baseline context only. PR #124 is **not a merge dependency**.
5. Check current main and open gameplay PRs before writing any shared file. At design time, Donut Q10/mana PR #123 was **already merged**; never assume it is still pending.

## Delivery sequence

One local coordinating agent may implement these steps **sequentially on separate small implementation PRs**, or in one bounded execution branch if its checks/evidence remain reviewable. Preserve one source of truth and report each stage's completion independently. Any unfinished stage must stay explicitly unchecked.

### A. Pure Rust playable environment (first mergeable unit)

- Write a small Bevy-free `AutoplayEnv` wrapping the current fixture/scene/combat state. Load validated existing `EnvKit`/`Fixture` combinations using the normal asset root; expose `reset(seed)`, `step(ActionV1)`, `observe()` and immutable episode diagnostics; use exactly the same 60Hz combat rules.
- Use a **Carl-only, single-fight** scenario first. Include at least `cp01_single_goblin` and `cp02_two_goblins` (or two honestly distinct existing legal layouts); a third targeted contact fixture is helpful. Do not use a generic scenario runner presumed to exist; the repo's existing scripted fixture runner is real.
- Implement the action mapping as the spec's `MultiDiscrete([3,2,2,2,2,2])` with real held-button edges. Start with `action_repeat=1`; support a documented repeat option only with correct press semantics.
- Clean terminal and truncation before automatic respawn. Reset constructs a **fresh** initial state and verifies initial hash parity. Unsupported fixture/layout/overflow throws, not silently simplified physics.
- Implement observation generation against a documented fixed feature schema and perception mask; test that the observation callback is pure and that hidden enemies/trigger timing never enter the policy observation. Keep complete state only for debug/metrics.
- **Acceptance A:** the same per-tick input sequence produces the same `state_hash`, gameplay events and outcome through the native harness as through the existing fixture path; repeated reset/seed parity and terminal states have unit tests.

### B. Native evaluation, baselines and reliable evidence

- Add a small CLI: enumerate supported scenarios, run one bot, evaluate a batch, replay/verify a stored episode, and benchmark release headless ticks/s. Use stable command conventions; document exact commands that work.
- Ship three reproducible baselines (`random_legal`, `approach_and_attack`, `reactive_survivor`), and an **event-driven** telemetry collector. Do not have heuristics read private timers, future scripted inputs, hidden foes or privileged `info`.
- Record versioned metadata, per-tick normalized action traces, ordered authoritative events, sampled/final game state hashes and actual terminal reason. Use seeded reproducibility; don't pretend a seed changes static fixture physics.
- Produce machine-readable per-episode + aggregate metrics (JSON and CSV), explicit success confidence interval, times/damage, action usage, enemy contact/death causes, no-progress, unreachable/softlock diagnostics where observable and replay pointers. If source events don't carry attribution, record `unknown` instead of guessing.
- Split truly different **scenario configurations** into development and hold-out evaluation; prevent exact duplicates masquerading as generalization. Pin both scenario roster and bot settings so an agent can compare two different game commits fairly.
- **Acceptance B:** a replay generated by a bot verifies identically in a fresh Rust process (same tick/event hashes); one intentionally corrupted action fails replay verification; one fixture overflow fails clearly; summary displays number of episodes and which cases were held out.

### C. Optional Python dependency bridge and a first real ML training run

- Build a separate tooling-only native binding with PyO3/maturin (preferred). Do not put Python/torch/Gymnasium into normal game Cargo or its runtime. Add a small Gymnasium adapter that conforms to the correct `reset`/`step` five-tuple, fixed action/observation spaces and `check_env`.
- Provide a reproducible local installation recipe and a smoke script: *build Rust binding -> run Python env checker -> evaluate a baseline -> verify its native replay -> train a small PPO model -> reload and evaluate*. All steps should run CPU-only without external accounts, services or assets.
- Use Stable-Baselines3 `PPO("MultiInputPolicy", env)` with bounded, configurable training steps. A **10,000 timestep smoke** is enough to prove training/save/load; it does not have to beat a baseline or succeed consistently. Seed Python/NumPy/PyTorch, environment and action sampling; persist version/config metadata alongside checkpoints.
- Test installed package behavior from outside the source tree to avoid false-positive imports, and document/measure extension/step overhead. Add batching only if justified by measurement.
- **Acceptance C:** a clean local setup can run `gymnasium.utils.env_checker.check_env`, at least one deterministic evaluation set, a bounded PPO training job and reloaded model evaluation; no training model weights or bulky traces enter git.

### D. Final integration and reproducible benchmark report

- Verify existing headless combat, demo, party and boss goldens are unchanged (or explicitly explain and migrate a *genuine* required change; default expectation is **no gameplay change**).
- Measure Rust-only and Python single-env simulated ticks/s; if it is slower than real-time, identify a bottleneck rather than promising a speedup. Record toolchain, OS, CPU, build profile, fixture/observation complexity and repeat count. Frame render timing is not training throughput.
- Run all appropriate project format, lint, unit, docs and Python tests; a new Python workflow can be added as an **isolated opt-in CI job** with pinned tooling, not a dependency of rendering/game packages.
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
python -m autoplay train --algorithm ppo --scenario cp01_single_goblin --steps 10000 --seed 42 --out artifacts/autoplay-01/ppo-smoke
python -m autoplay evaluate --scenario cp02_two_goblins --checkpoint artifacts/autoplay-01/ppo-smoke/model.zip --episodes 50 --out artifacts/autoplay-01/held-out
```

These are **interfaces to implement**, not commands available today. If actual packaging requires different commands, update both documents and tests with the verified alternatives. Avoid fake evidence or claiming a static seed is domain randomization.

## Non-negotiable test cases

| Test | Expected result |
| --- | --- |
| Same initial fixture and reset twice | Same starting hash, observation, event cursor and allowed actions |
| Same actions + same scenario from scratch | Identical authoritative ordered events, hashes and terminal outcome |
| Hold attack during multiple steps | One press edge until released and pressed again |
| Simultaneous buttons | Existing Rust action precedence used; no invented Python combat order |
| Detect defeat before respawn | One terminated episode, no automatic retry inside it |
| Timeout while alive | Truncated, not defeated; final observation retained |
| Reward from one defeated foe | Credited once; no bonus from repeated blood/GIF/audio frames |
| Hidden enemy / hidden trigger | Absent from policy observation; optionally visible in separate diagnostics |
| Locked special | No in-game power granted; attempted invalid action accounted for |
| Incompatible fixture/replay versions | Explicit rejection/migration needed |
| Native replay after Python/agent run | Same game event + tick hashes |
| Independent parallel envs | No cross-env mutable/global state; their seeds/counters stay isolated |
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
4. **Independent reviewer (top tier):** one focused architecture/design review of observation leakage, terminal semantics and event attribution; then standard code review and normal AGENTS.md merge rules.

No overlapping writes to `src/combat_scene.rs`, `src/corridor.rs`, `src/input.rs` or `src/party.rs` with the ongoing combat redesign. Prefer additive Rust modules and a separate optional tooling package. If the existing game API needs a change, isolate it in the smallest possible PR and coordinate its merge with the combat integrator.

## Agent report contract

Report: implemented units A/B/C/D, exact commands and results, final head SHA, stable schema versions, Rust and Python throughput figures, files/evidence links, any test not run, known cheating/observation limits, and next stage. Use the repo's PR body template (Why, Changes, Acceptance, Testing, Evidence, Performance, Rights, Risks, Next PR). **Do not label game balance, human difficulty or game fun "validated" by automated agents alone.**
