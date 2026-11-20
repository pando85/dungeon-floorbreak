# REWARDS-01 — Agent implementation handoff and verification matrix

**State: SPECIFICATION ONLY.** Owner's direction is to remove conventional item/gear management and reward expressive combat. This PR does not implement it. Read [REWARDS-01 spec](../01-design/combat-rewards-and-no-inventory-v1.md), [roadmap](../04-production/rewards-roadmap.md), [ADR-0010](../05-decisions/ADR-0010-combat-first-progression.md), `AGENTS.md`, [MVP controls](../01-design/mvp-controller-contract.md), [character horizon](../01-design/character-progression-horizon.md), [AUTOPLAY](../03-tech/autoplay-01-automated-gameplay-evaluation.md), AUDIENCE-01, CINE-01, CF02 and COMPANION designs **before any coding**.

## A. Global implementation contract

- Implementation: **100% existing Rust + Bevy**, fixed step, seeded randomness, Bevy-free pure gameplay domain. No ML code or Python in shipped runtime; do not start a second game engine. Version schemas.
- One authoritative defeat and no derived kill events from death visuals. Read the *current* `CombatEvent`, `DefeatCause`, `MoveId`, `ForceTier`, and hero attribution before extending. Do not pretend hypothetical enum names in this document already compile.
- No new persistent HUD bars, inventory screens, player attack buttons, consumable hotbar or sponsor UI at Floor 01 start. Existing `HudSettings` and native 640x360/400 captures remain the visual authority.
- Product decisions are accepted; numeric points, 8 s duration, effect strengths and card selection defaults are **experiments**, stored as JSON tuning with schema + validator. Roll back any failing option.
- Every implementation change is feature-flagged. Start with rewards **off** by default, including on `--demo`; opt in through an explicitly named scene/test flag, wired consistently to headless fixtures. Legacy `COMBAT_STATE`, camera, CINE and AUDIENCE goldens stay unchanged with flag off. Changes under flag on get independently versioned reward digests and replay evidence.
- Stable hero identities, **not controller slot numbers**, govern attribution. AI and human control must not create new wallets or clone a grant on swap. Team modifiers are never per-player duplicated. Enemy material/immunity and story capability gates are always stronger than a reward.
- Any effect must create an *observable combat decision* (push, timing, setup, survival), not merely inflate global damage. Keep authored floor navigation and boss tells intact.
- Coordinate with the live **CF02-00 #134** and **COMPANION-AI #129/#130/#131** agents. No simultaneous edits to shared combat, input, party, HUD or camera files without explicit API ownership transfer and rebasing.
- All PR/commit/doc/code text in English. No franchise-specific names baked into simulation IDs. Use source-backed chapter/lore evidence for eventual special objects; do not represent this concept as a franchise license.

## B. Event/versioned data proposal (compile only after reconciling source)

```rust
// Sketch only: resolve to existing combat/party types first.
struct SpectacleFactV1 {
    event_id: u64,
    tick: u64,
    encounter_id: u32,
    target_id: u32,
    actor_hero: Option<u8>, // 0 demolitionist, 1 caster
    helper_hero: Option<u8>,
    fact: SpectacleFactKind,
    source: SourceAuthority, // human, companion, authored environment
}

struct RewardStateV1 {
    per_hero_momentum: [MomentumState; 2],
    active_team_floor_modifiers: Vec<FloorModifierId>, // len <= 2
    stage_receipts: BTreeSet<StageClearId>,
    event_receipts: BTreeSet<EventId>,
    campaign_capability_refs: Vec<CapabilityId>,
    reward_rng_state: u64,
    reward_rules_version: u32,
}
```

Prefer existing domain IDs / ordered sets and structs from the live repo. Use at most one allocation-bounded receipt store per active floor/attempt or a proven equivalent; no indefinite unbounded event history. There is no separate persistent item instance, stack count or slot UI. A deterministic story item is a capability flag.

**Source-of-truth fan-out:** `CombatState` authoritative event -> one immutable normalized event stream -> REW-01 evaluator and AUDIENCE observer and CINE presentational signal. The reward domain may apply grant effects to combat only via a stable explicitly owned adapter; the HUD only reads reward snapshots. Do not put `AudienceState` or `HudRuntime` inside `CombatState`. If true numeric viewer thresholds become mandatory, implement a separate domain-owned audience projection and parity migration first (see `REW-06` below).

## C. Numbered delivery order with independent PRs

### REW-00 — This design/ADR PR

- Verify status against current main/merged contracts, add product policy and catalogue, dependency roadmap, agent-ready specifications, backlog and horizon.
- Ensure all new links exist; docs CI green; independent top-tier design review for conflicting input, AI, 1P/2P, story and audience assumptions.
- No gameplay modules, sprites or reward flag in this PR.

### REW-01 — Pure scoring/evaluator domain

- Own new `src/rewards/events.rs` and `src/rewards/spectacle.rs` (or the closest idiomatic module structure after source inspection). Pure inputs from existing accepted combat event types. Add a normalization adapter **only when its source-file owner has released the seam**.
- Data-driven scoring, 180-tick chain window, integer repeat damping, skill-category uniqueness, manual-vs-companion source distinction, target/event dedupe, bounded processing and fixed-seed proof.
- Test category mix, 3-second expiry, once-per-target explosion, wall knockdown vs harmless contact, authored anticipation not granting style, unknown instigator, two-hero contributions, no-gore-difference and changing pad/active hero ownership. Do not score CINE cosmetic triggers.
- Print separate, versioned `REWARD_STYLE_STATE` fixtures, never reuse `AUDIENCE_STATE` and do not change old hashes when off.

### REW-02 — Two small, real momentum effects

- Wait for REW-01 merged and compatible stable kick/pounce integration, especially CF02-00's authoritative file ownership. Domain state under `src/rewards/momentum.rs`; small scene adapter in a serialized PR.
- `RM-CARL-01`: +12% clamped impulse for next two landed kicks, max 8 s. `RM-DONUT-01`: one pounce adds at most six legal hold ticks, max 8 s (no pre-biscuit magic). All numbers configurable; include non-effect guard for immune/invalid contacts.
- Tests: no buff on a routine solo kill, cap per encounter, expiry during hero switch, companion eligibility, two controllers, boss caps, save/retry and no perturbation of critical material. Show actual real Bevy A/B evidence. Manual gamepad verdict required.
- **M1 may be validated once REW-09A telemetry lands; stage choices/sponsors are not prerequisites.**

### STAGE-BOUNDARY-01 (upstream, coordinate owner)

- Audit current demo progression/respawn, checkpoint state, room stage clear and floor exit; write the smallest authoritative `StageCleared` and `FloorExited` contract with unique clear receipt + snapshot integration. Existing route progression is not proof of this API.
- Tests must distinguish encounter, stage, floor, attempt; verify repeated stage re-entry, full restart and death. Assign file ownership to existing scene/checkpoint lead.
- This is a first-class dependency in the roadmap, **not a hidden responsibility** of REW-02.

### REW-03 — Snapshot and lifetime engine

- Typed reward `snapshot()/restore()` with fixed-tick momentum, floor modifier stacks/charges, authoritatively consumed offer receipt, RNG seed and dedupe.
- On respawn restore exact defined checkpoint snapshot with no duplicate reward. Stage clear retains floor modifiers; floor exit removes them; campaign capability flags remain. Feature flag off = identity.
- Test three distinct transitions: stage clear -> checkpoint death, floor boundary, complete replay from same seed, plus mixed swap ownership.
- Do not invent a general cross-session persistence layer.

### REW-04 — Floor modifier catalogue and offers

- Read `assets/data/gameplay/rewards_v0.json` with schema validation (exact file name subject to current pipeline conventions). Include minimum 3 *meaningful, actually supported* proposals from RF-01..RF-06 and 3-card no-duplicate offer logic. No invalid pre-story cards; if fewer than 3 legitimate cards, use a smaller offer or skip without fake filler.
- Exactly one team-owned selection per qualifying stage. At most two concurrent floor modifiers, choose replacement explicitly when at cap; never reroll repeatedly at same checkpoint. Log offer ID, choices, selection, resulting state and scene constraints.
- Scripted two-stage run tests: 1P and 2P, controller reassign, companion source, checkpoint retry, post-floor reset, seeded deterministic order. Balance the same-stage reward budget across 1P and 2P.

### REW-05 — CRT stage reward selection and score/choice evidence

- Reuse existing stage-end/CRT UI design, native pixel fonts and HUD setup. Do not copy external games' card artwork. Prove readibility at 640x360/400, real controller focus/highlight, reduced motion/high contrast, co-op choice ownership.
- Compare alternation of chooser per stage vs P1-only. Record the owner verdict before locking defaults. The AI never forces a selection. No pause in active enemy combat or cutscene collision.
- Stage-end optional style recap may be shown, but never invent a new permanently visible style counter or replace individual viewer plates.

### REW-09A — AUTOPLAY analysis, in parallel with REW-03/04

- Extend the existing AUTOPLAY Observation/reward telemetry with versioned, **non-privileged** reward observations where players would know them. Diagnostics may include privileged event details but not policy inputs. Reward-grant events are **not** an RL exploit objective by default.
- Repeat same seed sets disabled/enabled; report encounter success, style category entropy, duplicate payout count, damage, clear time, unusual action repeats, active modifier pick skew. Include anti-farm bots and multi-hit loops.
- Keep 01C/PPO optional. Preserve the existing AUTOPLAY baseline, release build tests and real simulation.

### REW-06 — Later sponsor eligibility, gated by story

- Requires canonical `broadcast_live` unlock and authored sponsor opportunity. No Floor 01 opening audience. Sponsor threshold is independent of HUD; implement domain-owned notoriety milestones and bounded sponsor cooldowns using the same source facts.
- If a *numeric* audience threshold is wanted, this PR must **first** produce a separate coordinated AUDIENCE-01 migration extracting pure scoring and introducing an authoritative projection with digest parity against the original observer. Never read a rendered viewer number.
- Test fake authored anticipation, humiliation, explicit special feat, uncapped spikes, replay restart dedupe, which hero attracted attention, separate sponsor vs momentum criteria. No actual crate/grant yet.

### REW-07 — Sponsor grant resolution

- Safe delivery time, exactly one result per eligibility receipt, curated categories: immediate supply, existing floor boon, narrative one-off. No inventory slot. Deferred grant stays pending if combat still active; no guaranteed drop from any fixed view count.
- If story content is not canon-approved, keep placeholder generic original fixture only. Floor boon must count against the existing two-modifier cap, never spawn independent stacking system.
- Verify co-op ownership and solo companion attribution without increasing reward budget; visuals are optional until logic passes.

### CAMPAIGN-SAVE-01 + REW-08 — Exceptional persistent objects

- Only after explicit campaign save contract and lore approval: a named object automatically unlocks a typed capability enhancement that survives floor transition. It cannot be removed/sold/equipped into armor slots.
- Example *mechanical pattern*: story-earned enhancement of existing punch collision, not the claim of an invented canonical item. Use existing character capability IDs and save migration. One story grant = one idempotent capability record.
- Assert that an item never appears before its story chapter; test old save compatibility and no regression to the single-special grammar.

### REW-09B — Human and automated acceptance / tuning

- Integrated 1P, local 2P and (when COMPANION 01E + AUTOPLAY-02a exist) AI synergy evaluation. Exercise 2 reward intensities plus disabled control. Record owner answers for fun, motivation, perception, fairness and stage-choice duration.
- Update the spec values/catalogue by evidence. A green unit test does not equal game-feel acceptance. Any reward that induces grind or passivity is deleted or retuned.
- Refuse wider rollout if effects overpower the boss, break actor gates, clash with dangerous camera cues or incentivize exploit loops.

## D. Fixture and regression matrix (minimum)

| Test case | Expected |
| --- | --- |
| `rwd_routine_goblin_no_momentum` | Basic kill fun unchanged; no buff |
| `rwd_carl_wall_launch_then_environment` | Meaningful categories + one eligible Carl chain |
| `rwd_donut_pre_biscuit` | Feline buff eligible; zero magic grants |
| `rwd_same_target_explosion_dedupe` | One target, one defeat credit, one receipt |
| `rwd_repeat_mitigation` | No exponential style farming by the same move |
| `rwd_gore_preset_parity` | 1–4 / reduced presentation changes zero reward state |
| `rwd_cine_no_reward_feedback` | Cinematic trigger never recursively awards style |
| `rwd_viewer_spike_not_mastery` | Audience anticipation spike cannot issue momentum |
| `rwd_swap_and_pad_permutation` | Hero 0/1 stable despite swap/2P slot changes |
| `rwd_ai_assist_no_double_pay` | Credit remains factual, team grants once |
| `rwd_checkpoint_reentry_idempotent` | Duplicate stage clear cannot reroll |
| `rwd_stage_to_floor_reset` | Floor modifier lost at floor exit, story skill retained |
| `rwd_offer_story_gating` | No mana card before biscuit or bombs before unlock |
| `rwd_stage_choice_native_deck` | Real controller, no clipping or focus deadlock |
| `rwd_held_out_autoplay` | Per-seed A/B numbers and exploit report |
| `rwd_legacy_off_golden` | All existing unrelated goldens identical |

Replace fixture names with actual registry naming style after inspecting the current suite; do not assert any already exists. Store expected versioned hashes with a short human-readable scenario report.

## E. PR evidence / review checklist

Each implementation PR description must state: 
1. `REW-XX` ID, target milestone, exact upstream merged PRs and file owner, flag/status.
2. Domain changes vs Bevy presentation-only changes, event version/schema migration and old-hash parity.
3. At least one fixture proof command + test result + supported artifact, a real native screenshot or short capture for visible changes, and a fixed-seed replay digest.
4. 1P, 2P, AI companion scope, exact story unlock guard and resource/boss/material safety.
5. AUTOPLAY diagnostics for changed reward balance or an honest dependency blocker.
6. Independent top-tier design / code review after implementation plus CI green; merged gameplay remains **game-feel review pending** until owner trial per `AGENTS.md`.
7. Updated [roadmap ledger](../04-production/rewards-roadmap.md) and [backlog](../04-production/backlog.md), not a private agent-memory note.

Recommended validation **when supported by the current repo**:
```sh
python3 scripts/check_docs.py
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
python3 scripts/validate_assets.py
```

Adapt real scenario CLI/CI names after inspecting them, and quote actual output. Do not mark reward effects 'proven fun' from headless metrics alone. This handoff is deliberately implementable by agents after the documented dependencies merge, without another speculative design round.
