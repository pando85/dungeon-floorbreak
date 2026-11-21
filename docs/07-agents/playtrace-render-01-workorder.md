# PLAYTRACE-RENDER-01 — Local-agent implementation work order

**Status: SPECIFICATION ONLY. No visual replay is implemented by this design PR.** Review round 1 resolved; see [technical design §8](../03-tech/playtrace-render-01-visual-replay.md#8-review-round-1-resolutions).
**Priority:** P1, after first recorded human CF02 playtests; implementation may start in parallel with LAB-02 and COMPANION 01B-wire. The user wants to watch the *actual human DEMO-02* replay through the Bevy renderer, diagnose Donut's poor participation and review goblin collision impacts. Delivery is in incremental code PRs; this is their integration contract, not a request to rewrite PLAYTRACE.

**Authority / read first:** [technical design](../03-tech/playtrace-render-01-visual-replay.md), [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md), [agent rules](../../AGENTS.md), [orchestration](agent-orchestration.md), [PT-D PR #161](https://github.com/pando85/dungeon-floorbreak/pull/161). Inspect latest main, all open PRs and uncommitted worktrees first.

## Objective and product cut

Build a **read-only, offline replay mode in the existing `floor01_scene` executable**. Input comes from an existing PLAYTRACE recording, never from real-gameplay pads; the *same* Rust deterministic sim drives the *same* Bevy actor/camera/HUD/VFX presentation. MVP: render a real solo Carl + AI Donut CF02 session, pause/step, see runtime overlays, and report replay tick + divergence/trust status. Then add restart, multi-episode seeking, per-event inspection, and optional frame/video export.

Do not mark this feature completed after writing a recorder/verifier wrapper that draws no original recorded player actions.

## Step 0 — Strict-verify precondition (no code, run once)

Before any checked viewing of the owner's `0bffd264` sessions, run strict `cargo run --locked --bin playtrace -- verify ID` for each of them on main `1dbd085e` (same source fingerprint as `0bffd264` when `rustc -V` is identical). Record PASS/FAIL and the verifier code per session in the R1 PR body, without session paths. A session that fails strict verify on its own source is UNSUPPORTED, never CHECKED. If the toolchain changed since recording, say so: strict is then impossible, and checked mode on the sim-pinned build is the only route. See [design §3-C](../03-tech/playtrace-render-01-visual-replay.md#3-replay-trust-classes--the-critical-compatibility-decision).

## Stage PTR-R0 — Shared replay cursor and policy (code PR A)

**Owner:** QA / simulation integrator. Files: `src/playtrace/replay.rs`, new `src/playtrace/cursor.rs` (+ tests), and the `pub mod` line/re-exports in `src/playtrace/mod.rs`. **Not** `src/bin/playtrace.rs`, so the only overlap with #161 is `mod.rs`. No Bevy dependency in the library. **No simulation file** (`src/combat_scene.rs`, `corridor.rs`, `party.rs`, `companion/**`, `cf02/**`, `donut_mana.rs`, `assets/data/**`, state-hash code) may change: R0/R1 must cherry-pick onto `0bffd264` (design §3-C).

Build (design §4 item 1, PTR-Q12):
- `ReplayCursor` in `cursor.rs` refactors today's `Replayer`; it does not rewrite it. It holds the journal iterators, drive, event hasher and cursor, episode, observation state and policy. It **does not own** `SceneSim`/`CombatState`; its methods borrow them.
  - `ReplayCursor::open(loaded, root, policy) -> Result<(ReplayCursor, SceneSim, Option<CombatState>, PartyTuning, Trust), VerifyFailure>`: preflight(policy), `rebuild`, `check_content`, start hash.
  - `cursor.next_tick(&mut SceneSim, Option<&mut CombatState>) -> Result<Option<TickOutcome>, VerifyFailure>`: controls with `at_t <= t`, pre-checks, step with recorded `hero` inputs, post-checks; `None` after the last tick once trailing controls are applied.
  - `TickOutcome { t, ep, et, applied: Vec<ControlEvent>, restarted, respawned_in_step, defeat_restart_in_step, drive: PartyDrive, src: [ControlSource; 2], events: Range<u64> }`. Surface `take_respawned()` and an in-step `combat.restarts` change instead of discarding them.
- `verify_loaded` becomes `open` + a loop over `next_tick` with owned locals. Its signature, `VerifyReport`, failure codes, output and every `tests_verify.rs` test stay unchanged.
- Policies: `Strict` is today's verifier. `CheckedCrossBuild` waives exactly one predicate, `manifest.source_fingerprint == SOURCE_FINGERPRINT`; an empty fingerprint is still rejected; a matching fingerprint reports STRICT (design §3-B). Checked mode exists only on the cursor and returns `Trust::CheckedCrossBuild`; `verify_dir`/`verify_loaded` stay strict-only and `playtrace verify` gets no compatibility flag.
- Scope: exactly what `verify_loaded` already supports (`RunKind::Human|ScriptedBot`, every reset source it rebuilds), no more. Keep the `ph` limitation explicit until truly re-derived. Do not send `EventRecord.d` through string parsing as gameplay semantics.

**Test (design §6 cross-build list):** sessions from the in-crate `live_tests` Rig, fingerprint rewritten with `tests_verify.rs::tamper_manifest`. Checked passes untampered; every existing tamper case fails in checked mode with the same code and tick; strict fails with `build_mismatch`; empty fingerprint rejected in both modes; one tampered AI-hero `eff` bit fails checked with `effective_input` at that tick (companion profile data is not in the content hashes, so it is caught only per tick). Golden hashes unchanged. Use **synthetic** fixtures only in committed test evidence.

**Deliverable:** a standalone testable replay engine; publish PR A with explicit limitations and API. This is **not** a visible replay feature yet; mark status accordingly.

## Stage PTR-R1 — Bevy visual playback (code PR B, P0 usability gate)

**Owner:** scene integration agent. Files: new `src/bin/floor01_scene/replay_view.rs`, declared as `mod replay_view;` in `src/bin/floor01_scene.rs` so it can use the binary's private items; confirm Cargo infers no new bin target from the directory (it has no `main.rs`). `floor01_scene.rs` edits are limited to argument parsing, the `mod` line, system registration and gating, and moving the presentation half of `build_scene` into a callable helper. No simulation file (R0 list), `src/hud.rs`, companion AI policy, terrain, player moves or command grammar changes.

Build (design §2, §4):
- CLI `--replay-session ID` and `--replay-compatibility checked`, ID resolved from the same local `--record-dir` / XDG base as `playtrace list`; an explicit session directory path may be supported without exposing it in PR evidence. `--replay-exit-at-end` for the smoke test.
- Sim settings come from the manifest only (deck from `reset.view_height`). Presentation flags `--scale`, `--camera`, `--gore-preset`, `--cinematic-finishes`, `--no-audio` are accepted at live defaults; overlay shows "presentation settings not recorded: defaults". Gameplay flags and every `--record*` flag are a parse error with `--replay-session` (list in design §2). `Recorder` is forced to `None`.
- Preflight failure opens no window: print `PLAYTRACE_REPLAY result=UNSUPPORTED code=…`, exit 1. Do not boot an unrelated demo or regenerate a fake recording.
- Move the cursor's `SceneSim`/`CombatState` into the existing `Sim`/`Combat` resources, which stay sole owners. Presentation setup = the presentation half of `build_scene` (`actors.animators`, `combat_art`, `death_presenter(...).with_cf02`, HUD flags); no second sim construction, or assert its state hash equals `manifest.reset.start_hash`.
- A replay `fixed_step` **replaces** the live one. Per tick: `next_tick`, then for each `TickOutcome` flag the same presentation calls as the live loop (`anim.step`/`step_art` or `reset`/`reset_art`, `death.observe`/`clear`, `hud.restart`, `controls.camera_cut`, `print_combat_events(Some(audio), Some(cine))`, `step_hud`). Split `step_combat` or inline its post-step half so `step_party` runs exactly once per tick, by the cursor.
- Do not register `capture_input`, `pad_connections`, live `fixed_step` or `sync_party_hud`. A replay HUD system builds `PartyView` from the cursor's `PartyDrive` (2P: Carl P1, Donut P2; solo: controlled hero P1, other Ai; default glyphs, `reconnect: None`, chips from `PartyTuning`). Viewer keys are keyboard-only, read in `Update`.
- Pause/resume and step one tick; restart is relaunching the process (in-viewer restart is R2). Show tick/time/episode and trust banner independent of the story HUD, **including a visible warning for cross-build mode**. Print one `PLAYTRACE_REPLAY session=… result=STRICT|CHECKED|DIVERGED|UNSUPPORTED ticks=… post=…` line; never a `PLAYTRACE_VERIFY ` line.
- Divergence (design §3-D): freeze the sim clock, disable step/play, keep drawing under a red `DIVERGED t=… code=…` banner ("this frame is not the recording"), print `result=DIVERGED`, exit non-zero on close. No continue flag.
- Use the gameplay event stream for VFX/audio triggers. If existing presenters advance particles by wall-clock while paused, **label** it as approximate; changing presenter systems for presentation determinism is out of scope for R1.
- Eager `read_session(dir, true)` is fine; no streaming work in R1.

**Automated acceptance:** Xvfb smoke per design §6 R1 row (record CF02 session → `playtrace verify` PASS → replay prints `result=STRICT` with `ticks`/`post` equal to the last `TickRecord` → one window capture at a fixed tick); parse tests for rejected flag combinations.

**MVP acceptance (manual):** watch an owner-recorded `--cf02` fight in a native Bevy window, with Carl/Donut and a goblin-body collision, and compare its original `TickRecord` / event stream with playback output, **on a build whose simulation equals the recording's**. For the owner's `0bffd264` sessions after #165 (or any other sim change) is on main, use the **sim-pinned replay build** (design §3-C): `git worktree add <tmp> 0bffd264`, `git cherry-pick <R0/R1 squash commits>` (only trivial `mod.rs` conflicts allowed), then `--replay-session ID --replay-compatibility checked`. Record the exact command sequence (no local paths) and `PLAYTRACE_REPLAY result=CHECKED` for one owner session in the PR body. The pinned worktree is local and never pushed. This is the user-visible milestone; don't defer it until video encoding or seeking.

## Stage PTR-R2 — Debug viewer and timeline (code PR C)

**Owner:** QA/UI adapter. New playback overlay and view-only input grammar, independent of gameplay HUD/command binding.

- Controls: restart from zero (in-viewer, sharing the presentation reset with seek); seek by global tick and by episode (re-sim from reset); 0.5x, 1x, 2x. Fast seeks may show progress; no fake random-access snapshot support. Avoid unlimited frame loops on the render thread.
- Event inspection: inspect selected actor's real simulated state/HP/target, Donut's control source and effective inputs, contact events, hit attribution, replay-vs-recorded event status. Viewer may display observation samples and diagnostic overlays; do not pretend `ObservationV2` is the full privileged internal AI plan, and do not parse unstable human-readable event debug text to infer HP loss.
- Reset HUD/VFX/presentation timing after any seek/restart so the displayed state is not stale. For approximate rendering during seek, label it.
- Tests: seek across death, checkpoint, solo swap, P2 join and AI decision; forward replay from seek reaches the same deterministic state/events as replay from zero.

## Stage PTR-R3 — Reproducible rendered frame export (code PR D)

**Owner:** rendering/QA. Reuse existing screenshot capture and the same cursor-driven Bevy presentation code.

- Optional `--replay-export-dir DIR`, `--replay-start-tick N`, `--replay-end-tick N`, `--replay-fps 60` (fixed 60Hz simulation). One deterministic simulation tick per output frame at the recorded deck; no dropped/duplicated ticks because of real-time frame pacing. Filename includes zero-padded global tick. Force CINE slow-mo off in export, or record it (FOLLOW-UP U-4).
- Write an export manifest including source session id (local only), original build, current build, trust class, rules/content hashes, presentation settings (or "not recorded: defaults"), tick range, image dimensions, renderer identity, frame checksums, `diverged_at` if any, and any known approximate presentation settings. No frame at or after a divergence tick. Only export after playback preflight. No exports inside git and no network upload.
- Optional documented shell helper for FFmpeg to generate MP4/GIF from the exported frames, **not** a required Rust dependency. Do not hard-require system FFmpeg for playing or exporting PNG.
- Tests: frame numbers, boundaries, image count, same-env checksums, crash cleanup, valid range, no file leakage, output size/performance. Measure throughput before promising full-session export (U-4). Do not promise cross-platform bit-identical pixels or lossless reconstruction of original audio.

## Stage PTR-R4 — Independent acceptance and owner handoff

- Synthetic and real human `--cf02` end-to-end, `--demo`, multiple attempts, solo Donut/AI Carl and real two-pad 2P **only when recordings of those exact modes exist**; record NOT RUN otherwise.
- One documented Donut inactivity window and one goblin-to-goblin hit; identify actual tick/event IDs. Verified source, reproducible render, visible HUD/terrain/animation/camera, and zero gameplay-state divergence.
- Frame time p50/p95/p99, memory high-water mark (decides whether streaming is needed), first-frame cost, 13k tick seek/export speed on named hardware; runtime with replay disabled behaves exactly as before.
- Run: `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets -- -D warnings`; `cargo test --locked --all-targets`; `bash scripts/ci_floor01_combat.sh headless`; `python3 scripts/check_docs.py`; applicable Bevy Xvfb captures. Do not claim success without actual results.
- Independent review per `AGENTS.md` / speed mode; human validates that the viewed recorded combat feels like their original session. Top-tier deferred reviews still follow the current temporary gate.

## Change isolation, ownership and dependency rules

- **PT-D #161** is open at authoring time. R0 overlaps it only in `src/playtrace/mod.rs` (`pub mod` line, re-exports); R0 does not touch `src/bin/playtrace.rs` or `tools/playtrace/**`. Rebase on whichever merges first; no force-push over another branch or parallel overwrite. #161's `verify.py` must keep accepting only `PLAYTRACE_VERIFY … result=PASS scope=full`, which checked mode never prints.
- **Donut-tuning PR #165** (`companion_ai_v0.json`, `src/companion/director.rs`) changes the simulation. Companion profile data is not in the content hashes, so `0bffd264` sessions pass preflight on a main with #165 and diverge at the first differing AI tick; that is correct fail-closed behaviour. Use the sim-pinned build (design §3-C). The orchestrator does not hold #165 for replay: if R0+R1 merges first, main can view those sessions in checked mode until the next sim change.
- **CF02, COMPANION, LAB-02** are changing gameplay code. Replay must pin compatibility evidence to the recorded version and fail closed when the simulation changes. Never alter old sessions to make them pass.
- Avoid changing `playtrace-session/1`, `autoplay-episode/1`, observation masks, event hashes, physics, existing goldens, controls, or default live recording policy.
- If replay reconstruction proves impossible for a lab/story session, document the exact reset/content gap and a narrowly-scoped follow-up. Do not fake its animation from event text.
- Never commit the owner's human session, any identifiable controller/personal metadata, exported videos or private file paths. Test records are synthetic and visibly labelled.
- Follow the game's fan-concept publication rules; a local rendered export is not a licence to publicly release fan media.
- Each code PR includes Why / Change / Acceptance Criteria / Commands & Results / Visual Evidence / Performance / Rights-Provenance / Risks / Next PR. The orchestrator batches implementation status into `docs/04-production/backlog.md` (speed mode).

## Definition of done

A human can select an existing recorded CF02 session, watch the original actions with the actual Bevy game art, pause and advance by tick, inspect why Donut did or did not help and what a goblin impact did, seek across retries, optionally export rendered frames, and obtain an explicit evidence-backed outcome: `STRICT`, `CHECKED CROSS-BUILD`, `DIVERGED` or `UNSUPPORTED`. No change to gameplay, live recording, privacy, or ML eligibility.
