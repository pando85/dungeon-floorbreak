# PLAYTRACE-RENDER-01 — Local-agent implementation work order

**Status: SPECIFICATION ONLY. No visual replay is implemented by this design PR.**
**Priority:** P1, after first recorded human CF02 playtests; implementation may start in parallel with LAB-02 and COMPANION 01B-wire. The user wants to watch the *actual human DEMO-02* replay through the Bevy renderer, diagnose Donut's poor participation and review goblin collision impacts. Delivery is in incremental code PRs; this is their integration contract, not a request to rewrite PLAYTRACE.

**Authority / read first:** [technical design](../03-tech/playtrace-render-01-visual-replay.md), [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md), [agent rules](../../AGENTS.md), [orchestration](agent-orchestration.md), [PT-D PR #161](https://github.com/pando85/dungeon-floorbreak/pull/161). Inspect latest main, all open PRs and uncommitted worktrees first.

## Objective and product cut

Build a **read-only, offline replay mode in the existing `floor01_scene` executable**. Input comes from an existing PLAYTRACE recording, never from real-gameplay pads; the *same* Rust deterministic sim drives the *same* Bevy actor/camera/HUD/VFX presentation. MVP: render a real solo Carl + AI Donut CF02 session, pause/step, see runtime overlays, and report replay tick + divergence/trust status. Then add multi-episode seeking, per-event inspection, and optional frame/video export.

Do not mark this feature completed after writing a recorder/verifier wrapper that draws no original recorded player actions.

## Stage PTR-R0 — Shared replay cursor and policy (code PR A)

**Owner:** QA / simulation integrator. Work in `src/playtrace/replay.rs` and focused new child modules + unit tests; touch `src/bin/playtrace.rs` **only** if a new read-only viewer preflight CLI is needed. No Bevy dependency in the library. Make a shared core for `verify_loaded` and viewer instead of cloning the reset/control/tick logic.

Build:
- A replay session loader/cursor with reset, `ControlEvent` timeline, per-tick recorded routed inputs, current `SceneSim` / `CombatState` and typed event/state view for Bevy. Clarify who owns mutable scene state; no shadow state with a second resolver.
- `Strict` checks identical to existing verification, `CheckedCrossBuild` checks all reproducible per-tick state/event hashes even when source fingerprint differs. The latter is NOT `playtrace verify` and NOT eligible for PT-D export. Fail on first divergence with tick, code and expected/actual fields; no lenient fallback.
- Correct handling of swaps, first drive change, joined second pad, AI source, pause boundaries, checkpoints, automatic respawns, held input edge state, session end and multi-episode control ordering. Keep `ph` limitation explicit until truly re-derived.
- A generic read-only API that lets a Bevy adapter drive one *existing* scene tick while getting a verdict. Do not send `EventRecord.d` through string parsing as gameplay semantics.

**Test:** strict verifier still rejects a different source build, unchanged golden hashes; new compatibility tests for old build, changed source but same sim and actual divergent sim; corrupted/truncated recordings fail. Use **synthetic** fixtures only in committed test evidence. Check the merge timing and file overlap with PT-D #161 before implementation.

**Deliverable:** a standalone testable replay engine; publish PR A with explicit limitations and API. This is **not** a visible replay feature yet; mark status accordingly.

## Stage PTR-R1 — Bevy visual playback (code PR B, P0 usability gate)

**Owner:** scene integration agent. Limited shared-file slot in `src/bin/floor01_scene.rs`; prefer a new `src/playtrace_viewer.rs` / viewer adapter as appropriate rather than adding thousands of lines to the already large scene binary. No changes to `src/combat_scene.rs`, companion AI policy, terrain, `src/hud.rs`, player moves or command grammar unless independently justified.

Build:
- CLI `--replay-session ID` and `--replay-compatibility checked`, ID resolved from the same local `--record-dir` / XDG base as `playtrace list`; an explicit session directory path may be supported without exposing it in PR evidence.
- Session preflight and clear unsupported-mode/error diagnostics; do not boot an unrelated demo or regenerate a fake recording if a session fails. Reconstruct the correct kit, CF02/party tuning, sprites, effective seed, gore/camera rules; reject incompatible content instead of silently drawing something else.
- On each fixed tick, feed a replay cursor, not live `PadSlots`/keyboard. Use the existing Bevy sync/render systems and make sure the scene and its rendering entities reset on retry/episode/scene changes. No second recorder. No real P2 join, AI decisions driven deterministically by the sim.
- Pause/resume, step one tick, restart from zero; show tick/time/episode and trust banner independent of the story HUD. **Include a visible warning for cross-build mode.**
- Use the gameplay event stream for VFX/audio triggers. Paused and stepped gameplay must not advance particles by wall-clock or emit repeated audio cues. If current presentation cannot guarantee this, identify it as approximate rather than claiming parity.

**MVP acceptance:** watch an owner-recorded `--cf02` fight in a native Bevy window, with Carl/Donut and a goblin-body collision, and compare its original `TickRecord` / event stream with playback output. This is the user-visible milestone; don't defer it until video encoding or seeking.

## Stage PTR-R2 — Debug viewer and timeline (code PR C)

**Owner:** QA/UI adapter. New playback overlay and view-only input grammar, independent of gameplay HUD/command binding.

- Controls: pause/resume; single simulation-tick step; restart; seek by global tick and by episode (re-sim from reset); 0.5x, 1x, 2x. Fast seeks may show progress; no fake random-access snapshot support. Avoid unlimited frame loops on the render thread.
- Event inspection: inspect selected actor's real simulated state/HP/target, Donut's control source and effective inputs, contact events, hit attribution, replay-vs-recorded event status. Viewer may display observation samples and diagnostic overlays; do not pretend `ObservationV2` is the full privileged internal AI plan, and do not parse unstable human-readable event debug text to infer HP loss.
- Reset HUD/VFX/presentation timing after any seek/restart so the displayed state is not stale. For approximate rendering during seek, label it.
- Tests: seek across death, checkpoint, solo swap, P2 join and AI decision; forward replay from seek reaches the same deterministic state/events as replay from zero.

## Stage PTR-R3 — Reproducible rendered frame export (code PR D)

**Owner:** rendering/QA. Reuse existing screenshot capture and the same cursor-driven Bevy presentation code.

- Optional `--replay-export-dir DIR`, `--replay-start-tick N`, `--replay-end-tick N`, `--replay-fps 60` (fixed 60Hz simulation). One deterministic simulation tick per output frame at the chosen virtual resolution; no dropped/duplicated ticks because of real-time frame pacing. Filename includes zero-padded global tick.
- Write an export manifest including source session id (local only), original build, current build, trust class, rules/content hashes, camera and gore settings, tick range, image dimensions, renderer identity, frame checksums and any known approximate presentation settings. Only export after playback preflight. No exports inside git and no network upload.
- Optional documented shell helper for FFmpeg to generate MP4/GIF from the exported frames, **not** a required Rust dependency. Do not hard-require system FFmpeg for playing or exporting PNG.
- Tests: frame numbers, boundaries, image count, same-env checksums, crash cleanup, valid range, no file leakage, output size/performance. Do not promise cross-platform bit-identical pixels or lossless reconstruction of original audio.

## Stage PTR-R4 — Independent acceptance and owner handoff

- Synthetic and real human `--cf02` end-to-end, `--demo`, multiple attempts, solo Donut/AI Carl and real two-pad 2P **only when recordings of those exact modes exist**; record NOT RUN otherwise.
- One documented Donut inactivity window and one goblin-to-goblin hit; identify actual tick/event IDs. Verified source, reproducible render, visible HUD/terrain/animation/camera, and zero gameplay-state divergence.
- Frame time p50/p95/p99, memory, first-frame cost, 13k tick seek/export speed on named hardware; runtime with replay disabled behaves exactly as before.
- Run: `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets -- -D warnings`; `cargo test --locked --all-targets`; `bash scripts/ci_floor01_combat.sh headless`; `python3 scripts/check_docs.py`; applicable Bevy Xvfb captures. Do not claim success without actual results.
- Independent review per `AGENTS.md` / speed mode; human validates that the viewed recorded combat feels like their original session. Top-tier deferred reviews still follow the current temporary gate.

## Change isolation, ownership and dependency rules

- **PT-D #161** is live at authoring time. Its `src/bin/playtrace.rs` and `src/playtrace/**` work may overlap PTR-R0. Prefer rebase/coordination after #161, not a force-push or parallel overwrite.
- **CF02, COMPANION, LAB-02** are changing gameplay code. Replay must pin compatibility evidence to the recorded version and fail closed when the simulation changes. Never alter old sessions to make them pass.
- Avoid changing `playtrace-session/1`, `autoplay-episode/1`, observation masks, event hashes, physics, existing goldens, controls, or default live recording policy.
- If replay reconstruction proves impossible for a lab/story session, document the exact reset/content gap and a narrowly-scoped follow-up. Do not fake its animation from event text.
- Never commit the owner's human session, any identifiable controller/personal metadata, exported videos or private file paths. Test records are synthetic and visibly labelled.
- Follow the game's fan-concept publication rules; a local rendered export is not a licence to publicly release fan media.
- Each code PR includes Why / Change / Acceptance Criteria / Commands & Results / Visual Evidence / Performance / Rights-Provenance / Risks / Next PR. The orchestrator batches implementation status into `docs/04-production/backlog.md` (speed mode).

## Definition of done

A human can select an existing recorded CF02 session, watch the original actions with the actual Bevy game art, pause and advance by tick, inspect why Donut did or did not help and what a goblin impact did, seek across retries, optionally export rendered frames, and obtain an explicit evidence-backed outcome: `STRICT`, `CHECKED CROSS-BUILD`, `DIVERGED` or `UNSUPPORTED`. No change to gameplay, live recording, privacy, or ML eligibility.
