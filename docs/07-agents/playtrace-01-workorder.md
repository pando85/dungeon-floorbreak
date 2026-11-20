# PLAYTRACE-01 — Agent implementation work order

**Task:** deliver the always-on, offline-first, versioned gameplay session recorder **before** COMBAT-LAB-01's first significant manual playtest.
**Design contract:** [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md), [AUTOPLAY-01](../03-tech/autoplay-01-automated-gameplay-evaluation.md).
**Status:** work requested / **no implementation in this PR**. The local implementers own code, tests and evidence; this PR defines the acceptance contract.

## Read and verify first

Read `AGENTS.md`, `docs/07-agents/workorders.md`, `src/input.rs`, `src/party.rs`, `src/party_scene.rs`, `src/corridor.rs`, `src/combat_scene.rs`, `src/autoplay/replay.rs`, `src/autoplay/observation.rs`, `src/bin/floor01_scene.rs`, COMPANION-AI work order and AUTOPLAY-02a spec in the unmerged design branch if needed. Trace **where the actual live game decides slot and effective hero inputs and executes one authoritative 60Hz step** before proposing any recorder hook. Check open CF02/AUTOPLAY/companion PRs for shared-file ownership before modifying runtime code.

## Implementation PR sequence (one mergeable subsystem per PR)

### PT-A — Recorder codec and durable journal (P0, no external gameplay deps)

Owner: `src/playtrace/**` and fixture/test artifacts. Implement `SessionManifest`, `EpisodeBoundary`, `PartyTransition`, `SessionControlEvent`, `RecorderStatus`, typed and schema-versioned streams, stable exact normalized action codec (including analog, no synthetic button-repeat edges), append-only compressed chunk writer/reader, checksums, 60-tick hashes, atomic completion and crash recovery. Add strict schema/config mismatch errors; use deterministic codecs for game data, not presentation/GPU output. Document feature flags and storage path. Provide a deterministic fake party producer to prove two independent input slots, action edges, reset/retry and corruption detection.

Acceptance: unit tests for bit-exact actions, ordering, duplicate/missing tick rejection, same-seed double-run, completed/aborted manifest, unreadable directory, full queue, killed writer/crash, checksum corruption and empty session. Capture-disabled producer gives identical logical game events. No replay schema v1 change.

### PT-B — Actual game recording by default (P0, required before owner play)

Owner: narrow live `floor01_scene`/party adapter, recorder initialization and status indicator, **without** rewriting CF02/companion input semantics. Start before first gameplay tick in `--demo`, `--cf02`, normal Floor 01, test fixtures and COMBAT-LAB rooms as each exists. Capture both slots, effective hero inputs, AI/human policy source/ownership, analog value, held vs pressed, swap, damage/collision and stage/checkpoint/retry/pause/quit boundaries. Include command request/accepted/refused when integrated. Separate global tick from episode tick. `--no-record` must be deliberate and visibly reported; default on in dev/QA runs. Storage failures mark partial files as incomplete with status and keep gameplay running (strict lab mode may fail early).

Acceptance: launch a live session, manipulate both heroes, swap, pause, die, retry, exit; inspect multiple episodes under one session and validate **every** fixed tick captured once. Compare capture enabled/disabled on identical deterministic input schedules; world/state hashes equal. Include two real-pad tests when hardware available, with unavailable hardware listed rather than fabricated.

### PT-C — Party legal observations, snapshots and fresh-process verification (P0 for ML claim)

Owner: shared observer data + verifier, in coordination with AUTOPLAY-02a and COMPANION-AI; **do not** change `autoplay-episode/1` ActionV1/ObservationV1. Define versioned `PartyObservationV1` with camera-correct legal view, hero-specific self/resources, visible threats, ability mask, input ownership and command feedback (when available). Capture `o_t` **before** live decision and `o_(t+1)` after step, while preserving matching actual action and causal events. Keep unseen HP/cooldowns/AI internals and future data in a separate privileged diagnostics stream. Add initial exact reset descriptor, mandatory 60-tick and boundary hashes; add full snapshots at transitions + 300-tick cadence if real game state serialization supports it, otherwise `snapshot_support=reset_only` with test coverage. Verify by replaying through the actual party simulator in a fresh process; preserve AI inputs exactly without relying on a current mutable policy, and separately compare recorded policy decisions when policy/model is available.

Acceptance: unit visibility differential tests, 1P Carl and 1P Donut with AI swapped, 2P both slots, per-hero observation/action pre-tick alignment, content/build mismatch rejection, deterministic world and events, after-destruction reset, negative replay tests. Required physical devices marked pending until tested. No false claim that offline replay reproduces rendering.

### PT-D — Exporter, human dataset quality and BC smoke (P1 after PT-C)

Owner: offline `tools/playtrace/**` or `tools/autoplay/**` adapter, without Python in runtime Cargo. Build `list`, `inspect`, `verify`, `stats`, `export-transitions` and `export-bc` CLI paths in established style. Preserve sessions/failed attempts, provenance and story/test overrides. Export only verified and eligible human-sourced actions, with per-tick legal `(obs, action, next_obs, terminal)` and optionally named reward/event facts. AI demonstrations are separate, never relabelled human. Support actor/hero filters and adjacent trajectory context. Partition datasets by full session/player pseudonym and actual scenario variants. Add schema migration or explicit unsupported-version failure. Include a tiny real human-driven capture end-to-end smoke; if unavailable use a synthetic fixture **clearly labelled synthetic** until human acceptance.

Acceptance: every dataset row maps to an original source tick/hash/action; no duplicate ticks; legal action, masks and observation timing verified; zero split leakage; corrupted/incomplete/partial observation sessions rejected with reason; successful simple imitation loader/train/validation; actual resulting metrics, not invented policy quality. Keep private diagnostics out of policy inputs.

### PT-E — Owner capture acceptance / performance evidence

Use a named actual PC and optionally Steam Deck-class hardware; measure p50/p95/p99 frame time, dropped/missing capture ticks, disk throughput and serialized bytes per minute under normal fight and heavy destruction with capture on/off. Record before/after comparison; no unsupported 60fps promise. Confirm saved sessions can be listed, inspected, locally deleted and exported intentionally. Owner performs a real controller attempt with capture status visible and a separate fresh-process verifier. Bugs discovered here are fixed in the owning PR.

## Ownership, dependencies and non-goals

PT-A and LAB-00 scenario authoring may run in parallel. PT-B depends on PT-A and one stable live input adapter. PT-C depends on PT-B plus AUTOPLAY-02a/COMPANION shared observer contracts where necessary. PT-D depends on PT-C. Do not require full PT-D to build greybox rooms; **do require PT-B before calling a playtest recorded**, and PT-C/D before calling a dataset ML-ready.

*No change to existing gameplay balancing, input grammar, reward mechanics, party swap authority, enemy behavior, default rendered art or story powers.* If additional event fields are needed, propose the smallest event-contract extension in the owning combat/companion PR. If dependency conflicts exist, record a narrow integration issue rather than guessing fields.

## Evidence bundle and PR checklist

Each PR must include Why / Change / Acceptance Criteria / Commands & Results / Visual Evidence (when applicable) / Performance / Risks / Next PR / Rights & Provenance. Use `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`, `bash scripts/ci_floor01_combat.sh headless` as applicable and report actual execution. Preserve every legacy deterministic fixture hash. Record exact commit, flags, seed, scenario and asset version for live evidence; never quote a passing test not executed.

**Definition of done for the program:** owner can start a game normally, play with Carl and Donut, die and retry, finish, and find **one local session with all actual actions and events**, replay it from a clean process, get an explicit verification result, inspect observations for human actions and export a valid offline dataset without fabricating lost data. Any missing feature must be surfaced as a failed gate, never papered over.
