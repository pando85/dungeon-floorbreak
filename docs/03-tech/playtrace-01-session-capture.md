# PLAYTRACE-01 — Record every gameplay session for deterministic replay and human-demonstration ML

**Status:** implementation-ready design contract, 2026-11-20. **Not implemented in this PR.**
**Owner decision:** Every local development/playtest gameplay session, including the original demo, COMBAT-LAB-01, solo Carl, solo Donut, AI companions, two-pad co-op, retries and sandbox tests, must be recorded by default. This means authoritative game data, **not video capture**. No manual "start recording" step is acceptable in development/playtest builds.

**Priority:** P0 infrastructure prerequisite to the next substantive owner playtest. This recording design does not grant story-locked powers in the campaign and does not introduce ML policy execution into gameplay.

Related authority: [AUTOPLAY-01](autoplay-01-automated-gameplay-evaluation.md), [companion AI](companion-ai-command-system.md), [test strategy](testing.md), [MVP controls](../01-design/mvp-controller-contract.md), [COMBAT-LAB-01](../01-design/combat-lab-01-advanced-playtest.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md).

## 1. Why the existing replay is insufficient

On main (2026-11-20), `src/autoplay/replay.rs` implements `autoplay-episode/1` (header + actual per-tick ActionV1 + rolling event/state hashes at 60-tick checkpoints + end), and `src/autoplay/scenario.rs` implements Carl-only fixtures. This works for verified one-actor automated tests but is **not** a universal recorder of live Bevy input, per-player-slot actions, AI-controlled Carl/Donut, context commands, physics across multiple rooms, checkpoint/retry boundaries or observation-before-action for human demonstrations. `src/party_scene.rs` already exercises `PartyState`, `SceneSim` and `CombatState` with `SlotInputs`: reuse that **one authoritative gameplay simulation**, not a parallel ML-only approximation.

Never silently repurpose `autoplay-episode/1` or overwrite its goldens. Define `playtrace-session/1` for continuous live gameplay with episode and stage segmentation; provide a version-aware bridge/exporter to existing AUTOPLAY tooling. A future `autoplay-episode/2` party format is owned by AUTOPLAY-02a; do not finalize a competing game simulation API.

**Three required products of one recording:** (1) a replayable action/event journal; (2) a human-perceived observation/action transition stream for offline learning; (3) provenance, evaluation and quality annotations. Video is optional, never a required training artifact.

## 2. Coverage and lifecycle (non-negotiable)

- **Always-on in local dev and QA/playtest profiles**, including `--demo`, `--cf02`, default `floor01_scene` gameplay, named fixtures when driven live, COMBAT-LAB sandbox/stress and story routes. Headless AUTOPLAY continues to use its existing recorder; it can optionally export the same training transition schema. Do not change public release/user-consent behavior by implication: a future distributed build requires explicit, understandable local recording controls and separate affirmative consent before any sharing. No uploads in this work.
- Start a session atomically **before the first simulated gameplay tick**. Record session ID, initial seed, build/assets and capability configuration. A session persists across scene transitions, human/AI control swaps, checkpoint restores, retries, character deaths and pauses; each attempt/encounter has an independent `episode_id` and authoritative reset boundary. Preserve the failed attempt, not just the winning one.
- Each session has a durable, ordered sequence of **ticks** while simulation advances and **control events** (pause/resume, menu time, device join/leave, session end, explicit abandon, hero ownership, checkpoint, level change, option change, story/test unlock). Paused real-world time is metadata; it does not secretly advance simulation ticks.
- Capture **both player input slots** on every tick, including neutral input, and each hero's **actual effective input** with source `human_p1 | human_p2 | companion_ai | scripted_bot | none`. Log `PartyState` ownership, hero identity and source policy/version, contextual command intent, command resolution/refusal, pending input buffers, special/ability availability, and solo swap handling. Distinguish *device request*, *normalized logical intent*, *accepted game-domain action* and *actual hero input*; never mistake AI for P2 or attribute AI kills to P1.
- Record from the **single final authoritative fixed-update dispatch boundary** after device adapters and AI generate inputs but before `step_party` consumes them. Record the **pre-step observable state** and the exact action supplied for tick t; after stepping capture post-step legal observation, ordered events, termination and state hash/checkpoints. Write a test that proves there is one logical gameplay step and one recording per tick even when Bevy render frames are skipped or duplicated.
- A test replay is considered **faithful** only if it recreates the correct party state/ownership, geometry, hazards, enemy AI, ability resources, enemy deaths, stage resets, all ordered gameplay events and agreed authoritative state hashes. Rendering output is not expected to hash identically.
- Recorder is passive. It must not change RNG order, hit timings, combat results, camera simulation, input edge detection, action buffer aging or AI decisions. No recording-to-gameplay feedback cycle.

## 3. Data contract: session, episodes, ticks and observations

All schema names are **proposed v1 defaults**, versioned independently. Use stable integer tick IDs and stable gameplay entity IDs scoped by session/episode; events carry a deterministic event ID, tick, source actor, target, cause and contact/impact attributes **only when truly observable**. Unknown attribution stays `unknown`, never guessed. Use named structured versioned enums/capabilities; avoid encoding spell names or franchise names in the generic recorder.

### 3.1 Session manifest (mandatory)

`manifest.json` stores:
- `schema`, `session_id` (locally generated random non-PII), `run_kind` (`human | scripted_bot | learned_policy | mixed`), `started_at_utc` (optional wall clock, diagnostic only), `build_commit`, dirty/build fingerprint, binary/toolchain identity and platform/build profile.
- `game_ruleset_id`, enabled feature flags (including `cf02_rules`), `tick_rate_hz`, content/asset/level hashes, recorded kit/scenario + variant ID, platform layout resolution, camera ruleset and RNG algorithm/seed **plus per-subsystem PRNG state when nondeterministic state cannot be reconstructed from seed alone**. Distinguish requested and effective seed.
- `action_schema_id`, `observation_schema_id`, `event_schema_id`, `snapshot_schema_id`, `party_schema_id`, `recorder_schema_id`, training extractor version, and exact session capability loadout/story-state/test-override flags.
- `mode` (solo Carl human, solo Donut human, local co-op), player slot mapping, input adapter versions, analog deadzone/quantization profile, assist/autoaim/accessibility settings, companion policy/profile/version/kill policy, AI internal RNG/blackboard initialization as required for replay. Never store a hardware serial, account ID or raw arbitrary device label.
- `status` (`recording | completed | abandoned | crashed_recovered | incomplete | corrupt`), `capture_complete`, `replay_verified`, `training_eligible`, `quality_flags`, chunk checksums and export lineage. Mark dirty/unknown builds as `replay_unverified` until verified with the corresponding executable.
- No mutable game options may change without an ordered `config_change` event. A change that alters simulation or schema starts a new segment with a recorded state boundary; never silently mix action schemas in one episode.

### 3.2 Ordered journal

`events.jsonl` for control/lifecycle, and compressed chunked `ticks` for actual 60 Hz transitions. Each tick must preserve:
1. `t` monotonic global fixed-update tick, `episode_id`, `episode_t`, `stage_id`, `encounter_id`, and `pre_state_hash` (or prove reconstructed against the previous checkpoint).
2. **Pre-decision observations** for each hero/player policy: exact `ObservationV*` values and masks (`visible_enemies`, hazard visibility, ability/action availability, known resources, screen-space camera, status/telegraphs), generated by the *same visibility rules and input timing* the agent would have when acting. This is the policy input `o_t`. Multiple actors share a camera in 2P but have distinct self vectors; store both. Include an `observation_is_partial` flag if support is missing. Do **not** leak hidden HP/cooldowns, future trajectories or unseen entities into policy input.
3. Both **requested per-slot** `RawInput` and **effective per-hero** input after ownership/commands, including axis values without lossy re-quantization, held buttons, down/interact/special/swap (as implemented), exact press-edge semantics and explicit `available` mask. Store frame-action schema version. IEEE-754 bit-preserving encodings or a validated deterministic canonical fixed-point mapping are acceptable; generic `[-1,0,1]` compression is **not** acceptable for analog axes.
4. Human/AI control source and policy version for each hero, ordered command requests/resolutions, and a marker if a requested action was rejected/cancelled/buffered by game rules. Optional AI decision diagnostics go into a separate privileged debugging channel, **never** into learning observation tensors.
5. **Post-step** observation `o_(t+1)`, ordered deduplicated authoritative combat/world events (hit, whiff when derived, damage, death, contact, terrain destruction, hazard, hero hurt, viewer/reward event if unlocked, spell cast, mana change, spawn/despawn, actor ownership, AI order status), terminal/truncation/segment transition flags and `post_state_hash`. `reward` is **not** an intrinsic human label; derive named reward profiles offline with their versions so evaluation criteria can change without recapturing games.
6. Source-of-truth actor/terrain state needed to analyze causality, from authoritative events + periodic serializable snapshots and versioned simulation assets. Full world-state is a **privileged diagnostic**, not policy observation; keep separate streams.

Suggested conceptual transition:
```json
{"t":120,"episode_id":"ep-2","obs_schema":"party-observation/1","obs":{"carl":"<versioned tensor>","donut":"<versioned tensor>"},"slots":{"p1":"<full normalized input>","p2":"<full normalized input>"},"effective":{"carl":{"source":"human_p1","action":"<exact>"},"donut":{"source":"companion_ai","policy":"companion_v0","action":"<exact>"}},"events":["<ordered event IDs and payloads>"],"post_state_hash":"<hex>"}
```
This example is a *sketch*, not a substitute for the eventual JSON Schema/typed Rust model; specify exact tensors/fields in the implementation PR and commit golden encoded samples. Use chunked compression for large tick arrays; validate complete ordering and checksums before export.

### 3.3 Snapshots, recovery and boundaries

Persist an initial authoritative **snapshot or sufficient reset descriptor** before tick 0, verified against a fresh reset hash. Commit complete serializable snapshots at every scene entry, checkpoint, retry, character-control mode change requiring a new episode and every **300 simulated ticks** (5 s at 60 Hz) when serialization is supported. Include active enemies, AI state, health, mana, timers, collider/terrain mutations, hazards, camera simulation state, active input buffers, PRNG state and every gameplay-affecting toggle. If full-state serialization is not yet available, preserve exact input tape + versioned reset descriptor + observation/event stream and mark `snapshot_support=reset_only`; *do not claim mid-run snapshots work*. Continuous full-state every tick is not a prerequisite if per-tick observations/actions/events + replay or snapshots reconstruct it.

Checkpoint hashes at least every **60 ticks** and at every boundary; separate state hash, ordered-event hash and chunk hash. Record **logical event ordering** (pre-state, action, update, post-state) to avoid off-by-one and AI action causality errors. Use an append-only journal with per-chunk checksums, temp-file + atomic completed manifest rename, periodic flush and recovery scanning. On crash, keep valid complete prefix as `crashed_recovered`, last complete tick and a `truncated` terminal; never invent missing input/actions.

### 3.4 Training transitions and leakage prevention

Exporter produces immutable, content-addressed datasets with rows `(session_id,episode_id,t,hero_id,controlled_by,o_t,availability_mask,a_t,o_t+1,done,outcome,event_refs,labels,split)`. It is **human-imitation eligible only if the selected hero's action came from a real human** and the observation preceded that action; AI companion actions remain separate annotated samples, not impersonated demonstrations. Human-controlled Donut and AI Carl must be as supported as human-controlled Carl and AI Donut.

- **Behavioral cloning:** human `(observation, legal action)` with action timing and contiguous trajectory context; retain successful and failed attempts, idle, misses, cancel/press edges and recovery from damage. No highlight-only selection. Preserve sequential context even if an offline trainer subsamples.
- **Offline RL / reward learning:** `(obs, action, next_obs, terminal, event facts)` plus separately versioned reward functions. A player liking a kill is not inferred from score/finish/slow motion.
- **Companion imitation:** filter actual human-controlled Carl/Donut intervals and pair with legal companion observations/actions; evaluate candidate policy with the same rules and role. Do not clone erroneous P2/human kill ownership or private debug facts.
- **Diversity and difficulty:** retain scenario/seed, move timing, death cause, damage, retry sequence, enemy archetype and strategy fingerprints; aggregate by comparable ruleset. Human notes and per-room 1–5 ratings are *optional explicitly authored labels* linked to tick ranges or encounter IDs, never auto-generated ground truth.
- **Train/dev/holdout split is by full session + player pseudonym when supplied + level variant/ruleset**, not by individual ticks. No adjacent ticks or retries from the same session in both train and test; report overlap and data leakage checks. Hold-out geometry/enemy configurations must actually differ, not just have new names.
- `observation_schema` changes require an explicit transform or retraining; don't silently pad/drop fields or treat the Carl-only `ObservationV1` as full co-op perception. Record timing and latency at tick level; do not infer real human reaction times from AI input unless controls/latencies are logged.

## 4. Recorder API and integration sketch

```rust
// Design only: names/signatures may be refined without changing semantics.
trait GameplaySessionRecorder {
    fn begin_session(&mut self, manifest: SessionManifest, reset: SnapshotOrDescriptor) -> Result<(), CaptureError>;
    fn begin_episode(&mut self, boundary: EpisodeBoundary) -> Result<(), CaptureError>;
    fn record_step(&mut self, transition: PartyTransition) -> Result<(), CaptureError>;
    fn record_control(&mut self, event: SessionControlEvent) -> Result<(), CaptureError>;
    fn finish_episode(&mut self, outcome: Outcome) -> Result<(), CaptureError>;
    fn finish_session(&mut self, status: SessionStatus) -> Result<(), CaptureError>;
}
```

Place a Bevy-free recorder core in a separate `src/playtrace/` module or an equally isolated tooling module, and a **thin adapter** at the live fixed-step boundary. Share canonical event types and hashing logic with `src/autoplay/replay.rs` where appropriate; do not import Bevy into `src/autoplay/` or train in the game process. Use the existing party simulation path. A headless playback CLI must support `list`, `verify`, `inspect`, `export-bc`/`export-transitions` and a minimal `stats` command; names may follow repo CLI style.

Default path: a dedicated local user-data / QA recordings directory outside tracked assets/source and outside git, organized by `session_id`. No network transport, no telemetry/analytics SDK, no recording of passwords, microphone audio, chat text, controller serials or free-form device identifiers. Generate an index/manifest for discovery, allow deleting sessions/exporting selected sessions, and provide a clear status indicator in dev UI/logs.

**Failure semantics:** gameplay remains available if storage is full/unwritable, but recorder must surface a visible, actionable error, mark the affected session `incomplete` and `training_eligible=false` and never claim that all gameplay was captured. A `strict-playtest` opt-in mode may pause/reject beginning a test when capture fails; normal runtime must not freeze gameplay to satisfy a recorder. Bound asynchronous write queues, expose backpressure/dropped-tick counters and test them; **never quietly drop inputs**. Keep `recording_enabled=true` by default in local development profiles and allow explicit `--no-record` with visible opt-out; tests with recording disabled must behave identically. Never silently enable collection/upload in a public distribution.

## 5. Quality gates before declaring recordings ML-ready

A session moves through **recorded → replay-verified → training-ready**. These are independent flags. Machine verification of these flags is mandatory; a complete file is not automatically a useful demonstration.

| Gate | Required result |
| --- | --- |
| Coverage | Zero missing/duplicate simulated ticks; both slots + effective per-hero inputs recorded (including neutral, simultaneous, repeated holds); scene changes, retries and interruptions intact |
| Validity | Manifest and every chunk checksum valid, monotonic tick/episode ranges, schema and asset fingerprints present, event IDs unique in scope |
| Replay | Fresh-process replay reproduces lifecycle, ownership, authoritative ordered events and state/event hashes at 60-tick checkpoints and boundaries |
| Observations | `o_t` exactly precedes action; policy observations mask invisible enemies/private timers; `o_(t+1)` is after the step; schema/availability matches actions |
| Causality | Human/AI/slot ownership and kill attribution preserved; AI companion never relabelled P2 or human |
| Recovery | Forced crash yields readable incomplete prefix; checksum corruption/missing tick rejected; recorder failure conspicuous and non-gameplay-affecting |
| Eligibility | A partial/missing observation or mismatched content/sim version excludes the segment from ML export, with exact rejection reason; raw record still retained for diagnostics |
| Isolation | Capture on/off produces identical sim hashes and gameplay event sequences under identical seed/actions; 60-Hz tick count unaffected by render FPS |
| ML smoke | Small dataset from **real** live-mode human-driven actions, not synthetic test traces claimed human; exporter performs shape/edge checks; simple BC train/eval consumes it without using private diagnostics; include negative tests |
| Privacy | All output remains local; opt-out and delete work; no device serial/account data; explicit separate export/share flow |

**Required test matrix:** keyboard, DualSense/Xbox logical adapter paths where available, held and sub-frame button edges, analog stick, solo swaps in both directions, AI Carl and AI Donut, two-pad 2P, death/checkpoint/restart, pause/unpause, disconnect/reconnect, changed level, enemy knockback/hazard destruction, ability/mana, inactive/missing special, long session, full disk, unwritable directory, crash recovery, concurrent independent recordings, same-seed replay on clean process. Unsupported current features must be listed as **pending** and not reported as passing.

**ML readiness rule:** no new multi-room human playtest may be declared a validated ML-capture session until the whole path `live input → disk → parse → fresh-process verify → export → sample inspection` passes. If full party env / serialized snapshots are not available, save complete raw recordings now and explicitly defer affected training subsets rather than fabricating support.

## 6. Workstream sequence and interfaces

- **PT-A — Core contract + disk writer/reader:** `playtrace-session/1`, manifest, episode lifecycle, chunking, exact action codec, unit tests, corruption/recovery. No game logic changes. Can start immediately.
- **PT-B — Live Bevy capture adapter:** hook one per-fixed-tick party step; automatic recording in dev/test including demo and LAB; every slot and hero source; settings/status/errors; no changes to controls. Needs the concrete party input pipeline reviewed, not COMPANION-AI completion.
- **PT-C — Observation/event/snapshot parity:** legal party observation `o_t`/post-state, authoritative event attribution, checkpoints and fresh-process verifier; synchronize with AUTOPLAY-02a and COMPANION-AI ownership contract. No new omniscient agent API.
- **PT-D — Offline export + smoke:** immutable datasets and leak-free split, BC extraction, metrics, schema migration tests, tiny real-session test; no production neural inference. Requires PT-C for training-ready party data.
- **PT-E — Playtest QA:** first owner session with capture status, replay verified and per-room human notes; measured recorder overhead on named hardware; unresolved schema/feature holes documented.

**Compatibility:** preserve all `autoplay-episode/1` fixtures/hashes and CF02 legacy off/on parity. If the main game or companion architecture changes mid-implementation, rebase and inspect effective fixed-tick input dispatch rather than guessing. New experiment-only unlocked abilities live behind explicit isolated capability fixtures, not the story route.

## 7. Defaults and open decisions (PT-Q)

| ID | Default |
| --- | --- |
| PT-Q01 | Dev/test recording on by default for **every** gameplay session; optional explicit `--no-record`. Public game sharing/collection out of scope. |
| PT-Q02 | One continuous session with separate episode/attempt IDs across death, restart and level change; paused time logged separately. |
| PT-Q03 | Per-tick both slot inputs, effective hero inputs + ownership, exact analog and held semantics; record observation **before** acting and next observation after. |
| PT-Q04 | Independent `playtrace-session/1`; existing `autoplay-episode/1` remains unchanged; versioned adapters only. |
| PT-Q05 | Initial reset descriptor + 60-tick replay hash checkpoints; serializable snapshots at transitions and every 300 ticks **when supported**, otherwise explicit reset-only limitation. |
| PT-Q06 | Local compressed chunked storage, atomic completion, crash-recoverable prefix, checksums, no upload. |
| PT-Q07 | Capture on storage error is incomplete, marked and visible; game continues; strict-lab mode may refuse unrecorded tests. |
| PT-Q08 | ML exporter accepts only verified, complete, human-labelled-by-source legal transitions; retains failures, separates AI provenance, no private policy leakage. |
| PT-Q09 | Session/episode/level variants and player pseudonym (if any) form the split boundary; no within-session tick leakage into holdout. |
| PT-Q10 | Rewards/score/visual spectacle are **derived**, not ground-truth human enjoyment; qualitative labels are explicitly user-provided. |

## 8. Explicit non-goals

No video/voice capture requirement; no live neural companion; no personal analytics service; no training cloud; no generic save-game replacement; no inventory/progression rewrite; no new physics or per-frame indiscriminate full-world dumps; no IP-uncleared asset releases. This document provides implementation requirements, not a statement that recording works already.
