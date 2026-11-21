# PLAYTRACE-RENDER-01 — Reconstruct and inspect recorded gameplay in Bevy

**Status:** design and implementation contract ONLY (2026-11-21; review round 1 resolved, see §8). No visual playback CLI, timeline, or export exists on main. **Priority:** P1 tooling for the next combat/companion playtest; do not block normal game startup. **Owner feedback:** real CF02 fights exposed weak companion participation and visually promising goblin collisions. We need to inspect the *actual human actions and rendered consequences*, not just inspect a headless simulator.

## 1. Baseline and source of truth

Read [PLAYTRACE-01](playtrace-01-session-capture.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md), [floor01-scene](floor01-scene.md), [COMBAT-LAB-01](../01-design/combat-lab-01-advanced-playtest.md), [companion spec](companion-ai-command-system.md), and [agent work order](../07-agents/playtrace-render-01-workorder.md).

- PLAYTRACE-A/B/C (merged PRs #143 and #156) write `manifest.json`, `control.jsonl`, and 600-tick `ticks-NNNNNN.jsonl` journals. `src/playtrace/codec.rs::TickRecord` contains exact per-slot/per-hero button+axis values, control sources, authoritative events, tick hashes and ObservationV2 where supported. The reset descriptor and control stream identify scene, party changes, restarts, pauses, checkpoint transitions and rule settings.
- `src/playtrace/replay.rs::verify_loaded` already reconstructs the real `SceneSim` / `CombatState` and compares the deterministic timeline. Its private `Replayer` (`control(&ControlRecord)` + `tick(&TickRecord, diag)`) already is a cursor; PTR-R0 refactors it (§4). It is **headless**: it never calls Bevy rendering, animation, HUD, camera presentation or audio. The recorded `ph` (party hash) is not fully re-derived. Its preflight rejects changed source fingerprints.
- `src/bin/floor01_scene.rs` is the existing Bevy presenter with render-loop screenshot facilities (`--capture-sequence`) for **scripted** fixtures, NOT recordings from human input.
- PT-D (PR #161, currently open) is an **offline ML/data exporter**, not a visual playback engine. Do not edit `tools/playtrace/**`, PT-D schemas or training eligibility to implement visual playback.
- The current recorder stores **reset-only** state: no mid-run snapshots. Several content-hash coverage holes are documented in PLAYTRACE-01; companion profile data is one of them (§3-B). Do not claim byte-identical audiovisual reconstruction just because a simulation hash matches.
- **Recent owner sessions are already recorded** on the older `0bffd264` build (per PT-D PR #161). Any new renderer build changes the coarse source fingerprint even when combat code is untouched, and the open Donut-tuning PR #165 changes the simulation itself. Supporting these pre-feature recordings is an explicit acceptance case; see §3-B and §3-C.

## 2. Product contract

Start with a local, offline-only, native Bevy **replay viewer** of an existing Playtrace session; no video capture is needed at record time. Opening a session replays the original timeline from tick zero and draws Carl, Donut, goblins, terrain, death/gore, camera, HUD and other existing presentation layers **through the same scene presentation code as live gameplay**.

Required initial CLI (final exact names may follow local argument conventions, but record any adopted change in the PR):

    cargo run --locked --release --bin floor01_scene -- --replay-session <SESSION_ID> --scale 3
    cargo run --locked --release --bin floor01_scene -- --replay-session <SESSION_ID> --replay-compatibility checked

The first command means strict same-build replay. The second is an **explicitly opted-in** cross-build visual reconstruction mode for pre-feature recordings; it never means `playtrace verify` passed under a mismatched source fingerprint (§3-B).

**Settings source (PTR-Q13).** The viewer derives everything sim-relevant from the manifest only: the deck comes from `reset.view_height`, never from the CLI; cf02, seed, demo/beat/boss/biscuit/companion/manual/autopilot come from the reset descriptor and feature flags. Presentation settings are **not recorded** today (FOLLOW-UP U-2). The viewer accepts the presentation flags `--scale`, `--camera`, `--gore-preset`, `--cinematic-finishes` and `--no-audio`, with the live defaults for `--cf02`, and shows "presentation settings not recorded: defaults" in the overlay and in the R3 sidecar. Gameplay-affecting flags are a parse error together with `--replay-session`: `--cf02`, `--cf02-seed`, `--demo`, `--demo-beat`, `--donut-biscuit`, `--fixture`, `--party-fixture`, `--companion`, `--manual`, `--deck`, `--scene`, `--autopilot`, `--lab-*`, `--capture-sequence` (R3 has its own export flags) and every `--record*` flag.

**Output line (PTR-Q14).** The viewer prints exactly one result line, `PLAYTRACE_REPLAY session=<id> result=STRICT|CHECKED|DIVERGED|UNSUPPORTED ticks=<n> post=<hex> [code=<code> tick=<t>]`. It never prints a `PLAYTRACE_VERIFY ` line (§3-B).

**Playback controls** (developer viewer, not game bindings): pause/resume; frame/tick step; show episode boundary and current `t` / `et` / seed / build / human-vs-AI ownership. In R1, relaunching the process is the restart; in-viewer restart arrives with seek in R2. Minimal keyboard shortcuts are acceptable, read in `Update`; keep all controller gameplay bindings unchanged. Pads are not read at all in replay mode (§4, binary side), so normal player/pad events cannot alter the reconstructed world or join P2. Replay viewer does not start another Playtrace recording: `Recorder` is forced to `None`.

**Next UX**: restart from zero, jump to a tick or event via replay-to-target from reset (no snapshot assumption), navigate between attempts/deaths, event bookmarks, speed 0.5x/1x/2x and an optional free debug camera. Viewer can compare an AI decision or contact with recorded `ObservationV2` and real event data without interpreting the debug-text `EventRecord.d` as a stable typed event schema. For now expose verified event labels and relevant in-simulation state from the matching simulator version; richer typed diagnostics come via a separate schema PR if needed.

**Export**: deterministic per-simulation-tick rendered PNG sequence at 60 Hz with a sidecar manifest (session/tick/episode/build/rules, presentation settings or "not recorded: defaults", verification class, checksums, FPS). An **optional external** FFmpeg wrapper can encode MP4 or GIF from that sequence; no system FFmpeg/runtime dependency in the Rust game and no claim of bit-identical audio/video. Exports must be opt-in and written outside git. Prefer an integration of existing `--capture-sequence` / frame capture infrastructure rather than a second renderer.

## 3. Replay trust classes — the critical compatibility decision

**A. Strict / same-source** (new recordings made with a renderer-enabled build): retain the exact existing `playtrace verify` contract, including schema, source fingerprint, rules, reset/content checks, every tick's `pre` / `post` / `eh`, effective hero inputs and event ordering. Fail closed on missing ticks, unsupported source, corrupted stream, unverifiable reset, and mismatched content. Report the party-hash limitation truthfully until fixed.

**B. Checked / cross-source** (older human CF02 sessions, PTR-Q02).

- **One waived predicate.** `CheckedCrossBuild` runs exactly the strict preflight, content, start-hash and per-tick checks of `verify_loaded`, with **one** predicate waived: `manifest.source_fingerprint == SOURCE_FINGERPRINT`. An empty fingerprint (pre-PT-C) is still rejected. The checks that stay on are: status/gaps/drops, schemas, ruleset, cf02 flag, kit/mana/cf02/party_tuning hashes, start hash, and per tick `pre`, routing, labels, stage, `o_t`, diag, effective inputs, `post`, ordered events, `eh`, `et`, plus `o_(t+1)` at boundaries. If the fingerprint matches, the result is reported as STRICT. Checked mode is never a downgrade.
- **ADR-0011 mapping.** ADR-0011 §3 says "a different build is `replay_unverified`". CHECKED CROSS-BUILD is `replay_unverified` with tick-by-tick parity evidence. It is not a new verification class, so no ADR change is needed.
- **Separate from verify at the type level and in output.** `verify_loaded`/`verify_dir` keep returning `VerifyReport` for strict only. Checked mode lives only on the cursor (§4) and returns `Trust::CheckedCrossBuild`. No code path prints a `PLAYTRACE_VERIFY ` line for checked mode; the viewer prints only `PLAYTRACE_REPLAY …` (§2). `playtrace verify` gets no compatibility flag. PT-D's `verify.py` (#161) accepts a session only on exit 0 plus `PLAYTRACE_VERIFY … result=PASS scope=full`, so it cannot accept checked mode as long as `verify_dir`/`verify_loaded` and their output stay unchanged.
- **Companion data is not in the content hashes.** `content_hashes` records only `kit`, `mana_tuning`, `cf02_tuning` and `party_tuning`, and `party_tuning_hash` encodes the `uncontrolled_hero` enum, not the contents of `companion_ai_v0.json` (`companion::profile::PROFILE_HASH`). Companion profile data is therefore not covered by preflight on recordings made up to PTR-R0. A changed profile (for example #165) passes preflight and is detected at the first tick where the re-derived AI input or state differs (`effective_input` or `post_state`). That fails closed and is sufficient; recording the profile hash for new sessions is FOLLOW-UP U-1.
- **Labelling.** Display an always-visible label such as `CHECKED CROSS-BUILD (NOT ORIGINAL-BUILD VERIFIED)` and record that status in exported sidecars. An original-build `playtrace verify` PASS, if available, is useful provenance but must not be faked. If the code is not compatible, state precisely that the old recording cannot currently be rendered and retain the data.

**C. Sim-pinned replay build (adopted PTR-Q11).** A recording is viewable on any build whose *simulation* equals the recording build's. Checked mode proves that equality tick by tick. To view sessions recorded on `<C>` after main's simulation has moved on (for the owner's sessions: `<C>` = `0bffd264`, and #165 moves the simulation), build a local, never-merged branch:

    git worktree add <tmp> <C>
    cd <tmp> && git cherry-pick <PTR-R0/R1 squash commits>
    cargo run --locked --release --bin floor01_scene -- --replay-session <ID> --replay-compatibility checked

To keep this possible, PTR-R0/R1 **must not edit any simulation file**: no `src/combat_scene.rs`, `corridor.rs`, `party.rs`, `companion/**`, `cf02/**`, `donut_mana.rs`, `assets/data/**` and no state-hash code. Their only shared edits are `src/playtrace/replay.rs` (+ new child modules), the `pub mod` line in `src/playtrace/mod.rs`, and `src/bin/floor01_scene.rs` (+ its child module directory). The R1 PR must cherry-pick cleanly onto `0bffd264`, with only trivial `mod.rs` conflicts allowed. It must record the exact command sequence it used, and record `PLAYTRACE_REPLAY result=CHECKED` for one owner session on that build. Owner acceptance on main is required only for sessions recorded on a build with the same simulation.

- **Precondition, run once before any checked viewing.** Run strict `cargo run --locked --bin playtrace -- verify ID` for the owner sessions on main `1dbd085e`, which has the same source fingerprint as `0bffd264` when `rustc -V` is identical (`0bffd264..1dbd085e` touches no file under `src/`, `assets/data/` or `Cargo.*`), and record PASS/FAIL per session. A session that fails strict verify on its own source is not shown as CHECKED. Report it as UNSUPPORTED with the verifier code. If the local toolchain changed since recording, say so: strict is then impossible, and checked mode on the pinned build is the only route. This step needs no new code and can run before PTR-R0 starts.
- **Sequencing with #165.** If R0+R1 merges before #165, main itself can view the `0bffd264` sessions in checked mode until the next sim change. The orchestrator does not hold #165 for this; the pinned build covers it.

**D. Divergence behaviour (PTR-Q15).** On `Err(VerifyFailure{code, tick})` from `next_tick`, the viewer:

- stops the sim clock permanently; step and play are disabled, and restart (relaunch in R1) is still allowed;
- keeps drawing the current state under a red `DIVERGED t=<tick> code=<code>` banner, with the text "this frame is not the recording";
- prints `PLAYTRACE_REPLAY result=DIVERGED code=… tick=…`;
- exits non-zero when closed.

Ticks before `t` count as matched. There is no flag to continue. A preflight failure opens no window: it prints `result=UNSUPPORTED code=…` and exits 1. R3 writes no frame at or after `t`, and its sidecar records `diverged_at`.

Never turn off divergence checks, silently fall back to autoplay, or rewrite manifests/hashes. Specifically do not use the existing party `ph` as proof of a verified party-state check until its derivation is implemented. The stronger path is a future *simulation fingerprint* decoupled from presentation-only source changes, designed separately and never assumed retroactively (FOLLOW-UP U-3).

**Visual fidelity level:** gameplay-authoritative state/event equivalence is the hard gate; pixel-exact presentation is only promised when verified independently with matching assets, presentation settings, deterministic event-driven animation/VFX and a known renderer environment. Replayed camera state should follow the same simulation and presentation rules (`camera_tick` is a pure function of sim, combat, interest and elapsed ticks). Presentation-only randomness needs event/session seeding or a recorded presentation stream; otherwise call the frame an approximate reconstruction. At variable speed, audio may be muted rather than pitch-shifted. Do not confuse a tick pause with a recorded wall-clock pause.

## 4. Implementation architecture

One authoritative replay driver, no second combat implementation.

1. **Seam (adopted PTR-Q12).**

   *Library side.* `src/playtrace/cursor.rs` (Bevy-free) refactors today's `Replayer` and does not rewrite it. `ReplayCursor` holds the loaded journal iterators, the drive, the event hasher and cursor, the episode, the observation state and the policy. It **does not own** `SceneSim`/`CombatState`. Its methods borrow them:
   - `ReplayCursor::open(loaded, root, policy) -> Result<(ReplayCursor, SceneSim, Option<CombatState>, PartyTuning, Trust), VerifyFailure>` runs preflight(policy), `rebuild`, `check_content` and the start hash.
   - `cursor.next_tick(&mut SceneSim, Option<&mut CombatState>) -> Result<Option<TickOutcome>, VerifyFailure>` applies the controls with `at_t <= t`, runs the pre-checks, steps with the recorded `hero` inputs, then runs the post-checks. It returns `None` after the last tick, once the trailing controls are applied.
   - `TickOutcome { t, ep, et, applied: Vec<ControlEvent>, restarted: bool, respawned_in_step: bool, defeat_restart_in_step: bool, drive: PartyDrive, src: [ControlSource; 2], events: Range<u64> }` reports the lifecycle. The cursor surfaces what the verifier currently discards (`take_respawned()`, a change of `combat.restarts` inside a step) instead of dropping it.

   `verify_loaded` becomes `open` plus a loop over `next_tick` with owned locals. Its signature, `VerifyReport`, failure codes and every test in `tests_verify.rs` stay unchanged. The cursor supports exactly what `verify_loaded` already supports (`RunKind::Human|ScriptedBot`, all reset sources it rebuilds), no more. AI inputs are re-derived by the existing `CombatState::step_party` director from `DriveChange` plus `live_tuning(manifest)`; no simulation file changes (§3-C).

   *Binary side.* `src/bin/floor01_scene/replay_view.rs` is declared as `mod replay_view;` in `floor01_scene.rs`. As a child module it can use the parent's private items (`Sim`/`Combat`/`Anim`/`Death`/`Controls`/`SceneCam`, `step_combat`, `print_combat_events`, `step_hud`, `camera_tick`). Confirm that Cargo does not treat the directory as a new bin target (it has no `main.rs`). The cursor's `SceneSim`/`CombatState` are moved into the existing `Sim`/`Combat` resources, which stay the single owners; no presenter system that takes `Res<Sim>`/`Res<Combat>` is rewired.
   - **Presentation setup.** It is the presentation half of `build_scene`: `actors.animators`, `combat_art`, `death_presenter(...).with_cf02`, HUD flags. Do not run a second sim construction. If code reuse makes that unavoidable, assert that its state hash equals `manifest.reset.start_hash`.
   - **Replay `fixed_step`.** It replaces the live `fixed_step` system (it is not a branch inside it). Per tick it calls `next_tick`, then runs the same presentation calls the live loop makes for each `TickOutcome` flag: `anim.step`/`step_art` or `reset`/`reset_art`, `death.observe`/`clear`, `hud.restart`, `controls.camera_cut`, `print_combat_events(Some(audio), Some(cine))` and `step_hud`. `print_combat_events` keeps its own `take_new_events` cursor, which is not in the state hash, so it and the cursor's event check run on the same state.
   - **Rewiring.** `step_combat` is split, or the replay system inlines its post-step half, so `step_party` is called exactly once per tick, by the cursor.
   - **Inputs and HUD.** Replay mode does not register `capture_input`, `pad_connections`, live `fixed_step` or `sync_party_hud`. A replay HUD system builds `PartyView` from the cursor's current `PartyDrive`: two-player is Carl P1, Donut P2; solo is the controlled hero as P1 and the other as Ai; glyphs at their default, `reconnect: None`, chips from the cursor's `PartyTuning`. A real pad's Start therefore cannot join P2, and a disconnect cannot trigger the D6 pause.
   - **`floor01_scene.rs` edits** are limited to: argument parsing, the `mod` line, system registration and gating, and moving the presentation half of `build_scene` into a callable helper.
2. **Clock.** Keep the authoritative simulation tick clock separate from render FPS, animation frames, and wall time. At 1x advance 60 recorded ticks per simulated second; in deterministic frame export, advance exactly one tick per output frame. Pausing freezes the sim clock. CINE slow-mo scales only `Time<Virtual>`, not the fixed step. Whether existing presenters (particles, animation) advance by wall-clock while paused is **labelled** in R1, not fixed: changing presenter systems for presentation determinism is out of scope for R1.
3. **Lifecycle.** Handle sessions with retries, automatic respawn, checkpoints, solo swaps, AI Carl or AI Donut, local 2P join/quit, source changes and session-end through the cursor's `TickOutcome` flags. Do not initialize a room directly to an arbitrary mid-session position without reproducing prior transitions. First eligible source: human `--cf02` session; then `--demo` and fixtures; gate story/combat-lab support by concrete successful reset reconstruction (lab reset remains a known PT-C review risk).
4. **Overlay.** Put replay debug presentation in a separate overlay resource/module: tick, time, actor authority, pause/speed, trust class, "presentation settings not recorded: defaults", selected event and clear cause-of-impact/target info. Existing HUD may remain visible but gameplay controls must not leak into it. Do not interpret `EventRecord.d` text to derive authoritative damage or state.
5. **Memory.** R0/R1 use the existing eager `read_session(dir, true)`; it is fine for a 13–15k tick session. Cap event/diagnostic views and keep no per-tick Bevy entity history. R4 measures the memory high-water mark; streaming or bounded buffering is opened only if that measurement shows a problem. Seeking (R2) is replay-from-reset and must display progress if it takes time; snapshots/indexing are a separate later optimisation.

## 5. Work packages and dependencies

| ID | Deliverable | Depends on | Done when |
| --- | --- | --- | --- |
| **PTR-R0** | `ReplayCursor` refactor of `Replayer`, strict and checked policies, tests | PT-C #156 | `verify_loaded` runs on the cursor with unchanged API/tests; checked mode passes the F-5 cross-build tests; no simulation file edited |
| **PTR-R1** | `replay_view` child module, CLI, play/pause/step, default camera/HUD/animations, trust banner | R0 | Xvfb smoke prints STRICT with matching `ticks`/`post`; one owner session CHECKED on the sim-pinned build (§3-C); no live input or new recording |
| **PTR-R2** | Restart, seek, bookmarks, speed controls, diagnostics overlay and per-hero event inspection | R1 | Can find a Donut inactivity window and a goblin-to-goblin hit without manual video scrubbing |
| **PTR-R3** | 60 fps offline PNG export with frame/tick metadata, optional FFmpeg helper | R1; R2 if timeline-range export requested | Same input yields same frame sequence in same build/environment; export is opt-in |
| **PTR-R4** | Controller/owner acceptance, audio and presentation parity, performance and regression evidence | R1-R3 | Verified replay of owner session, recorded known limitations, no production gameplay regressions |

Separate PRs are preferred for R0+R1 vs R2 vs R3; R0 alone is not a useful product milestone. An integrator may group R0 and R1 for an end-to-end usable MVP after independent review. **No hard dependency on PT-D #161, #165, ML training, companion redesign or LAB-02.** R0 does not touch `src/bin/playtrace.rs`, so its only overlap with #161 is the `pub mod` line and re-exports in `src/playtrace/mod.rs`; rebase on whichever merges first. Keep gameplay mechanics and live party rules unchanged.

## 6. Acceptance and tests

| Stage | Automated | Manual / evidence |
| --- | --- | --- |
| **R0** | Headless cursor tests (one Carl, solo swap, AI Donut, human Donut + AI Carl, party drive with P2 join, death/retry, scene break, pause event) from the in-crate `live_tests` Rig; the cross-build tests below; golden hashes and every `tests_verify.rs` test unchanged | none |
| **R1** | Scripted Xvfb smoke: (1) record a CF02 session with `floor01_scene --cf02 --autopilot --record --record-dir $TMP --exit-after-ticks 600`; if `--record` is refused for that run, write the session from the in-crate Rig through a `#[cfg(test)]` helper; (2) `playtrace verify` PASS; (3) `floor01_scene --replay-session ID --record-dir $TMP --replay-exit-at-end` prints `PLAYTRACE_REPLAY result=STRICT ticks=N post=<hex>`, where N and `post` equal the last `TickRecord`; (4) one window capture at a fixed tick. Parse tests: `--replay-session` with `--record` or any gameplay flag (§2) is an error | One owner session on the sim-pinned build (§3-C), with the strict-verify precondition result |
| **R2** | Seek across death, checkpoint, solo swap, P2 join and AI decision; forward replay from seek reaches the same deterministic state/events as replay from zero | Restart and seek across a respawn; HUD/scene reinitialize at episode transition |
| **R3** | Frame numbers, boundaries, image count, same-env checksums, valid range, no file leakage | Frames at the recorded deck (see below) |
| **R4** | Full regression set | Owner acceptance, performance, two-pad when recorded |

- **Cross-build (R0).** Cross-build tests record a session with the existing `live_tests` Rig, rewrite `source_fingerprint` with `tests_verify.rs::tamper_manifest`, and assert that:
  - checked mode passes as CHECKED (untampered);
  - every existing tamper case in `tests_verify.rs` (input, control, events, eh, eff, labels, obs, diag, content, rules, incomplete) still fails in checked mode with the **same code and tick**;
  - strict fails with `build_mismatch` (existing `a_wrong_build_is_rejected_and_an_identical_source_is_accepted`);
  - an empty fingerprint is rejected in both modes;
  - with a rewritten fingerprint, tampering one AI-hero `eff` bit fails checked mode with `effective_input` at that tick (§3-B companion coverage);
  - the viewer's `PLAYTRACE_REPLAY` output is not accepted by #161's `parse_verify_output` (assert on the line prefix; no Python in `cargo test`).

  One manual, non-CI check on the sim-pinned build of §3-C covers a real older build. Do not commit human Playtrace data to tests or Git.
- **Bevy/evidence.** Draw real CF02 frames and a short 1x GIF at 3 selected ticks (kick, body contact, Donut support or documented inactivity) **at the recorded deck**. A second deck only if a session recorded at that deck exists; a deck override is rejected (§2). Compare episode/tick, positions, camera bounds, sprites, environmental state and event counts with headless replay. Test frame export reproducibility in an environment with pinned software rendering, but distinguish identical in-environment pixels from cross-hardware guarantees.
- **Visual/interaction.** Start/pause/step (R1), restart and seek across a respawn (R2), two-pad session rendering without live input stealing control, one end-of-session clean exit, divergence banner and non-zero exit (§3-D); scene and HUD correctly reinitialize at episode transition. No extra combat event, RNG advance, companion move, camera teleport or replay writer output.
- **Performance (R4).** 10–15k tick owner-like session: first-frame latency, seek-from-start wall time, sustained playback frame-time p50/p95/p99, memory high watermark, export wall time and total disk bytes. Measure on named hardware; do not invent target frame numbers.
- **Regression (every stage).** `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `bash scripts/ci_floor01_combat.sh headless`, `python3 scripts/check_docs.py`; all existing goldens unchanged. Capture source/action version matrix and evidence in PR bodies.
- **Owner acceptance (R4).** One real human `--cf02` recording showing the same fighting sequence the owner remembers, including Donut's inactivity and a body collision, on a build whose simulation matches the recording (main or the §3-C pinned build); clearly differentiate what is verified, not verified and visually approximate. A second 2P physical-pad recording remains NOT RUN unless actually performed.

## 7. Non-goals and rights

No video captured during live gameplay, no upload/telemetry service, no ML inference, no automatic blame of companion decisions, no screenshot generation for copyrighted third-party assets, no change to combat damage or companion AI, no undocumented command rebinding, no compulsory periodic snapshots or rewrite of the PLAYTRACE session schema. Recordings and exports are private local files; **never** commit identifiable session data, user home paths or full personal recordings. The sim-pinned worktree of §3-C is local and never pushed. The public fan-concept assets remain under the repo's rights policy; a video export is not permission to distribute a game/trailer.

**Decisions for agents (adopted defaults pending evidence):** PTR-Q01: replay source of truth = existing sim + recorded routed inputs; Q02: default strict; explicit checked compatibility for pre-feature builds, one waived predicate (§3-B); Q03: PNG first, external FFmpeg optional; Q04: 60 fps one tick/output frame; Q05: seek from reset until snapshot work; Q06: pause/playback debug controls in viewer only; Q07: metadata always declares trust class and render settings; Q08: no networking or recorder loop; Q09: no change to PT-D eligibility; Q10: MVP targets CF02 human sessions first and adds other modes only as replay reconstruction is proven; Q11: sim-pinned replay build for recordings whose simulation main has left (§3-C); Q12: borrowed-state cursor in the library, `replay_view` child module in the binary (§4); Q13: sim settings from the manifest only, presentation flags at live defaults (§2); Q14: `PLAYTRACE_REPLAY` output line, never `PLAYTRACE_VERIFY` (§2); Q15: divergence freezes the sim and exits non-zero (§3-D).

**Follow-ups (separate PRs, not blocking):** U-1 record `companion_profile = PROFILE_HASH` in `content_hashes` for new sessions and check it only when present (coordinate with #161's eligibility code, which must ignore unknown keys); U-2 record presentation flags as non-hashed `presentation_*` feature flags; U-3 design the sim fingerprint separately, noting that a state-hash function change makes every older session diverge at the start hash; U-4 measure R3 throughput (`capture_sequence` settle frames per PNG) and force or record CINE slow-mo off in export; U-5 R0/R1 test sessions come from the in-crate Rig because PT-D `synth-fixture` refuses arena fixtures; U-6 crash-recovered sessions show `STRICT (prefix)` / `CHECKED (prefix)` and stop with "end of valid record", never DIVERGED.

## 8. Review round 1 resolutions

Independent top-tier review of PR #164 requested changes (2 blocking, 10 fix-now). Resolutions:

| Finding | Resolution |
| --- | --- |
| **B-1** no route to view `0bffd264` sessions once #165 lands | §3-C sim-pinned replay build (PTR-Q11): worktree at the recording commit + cherry-picked R0/R1; R0/R1 edit no simulation file; strict-verify precondition on main `1dbd085e`; #165 sequencing note. R1 acceptance in §5/§6 and work order |
| **B-2** R0/R1 ownership seam open, lib-side adapter cannot see binary items | §4 item 1 (PTR-Q12): `ReplayCursor` borrows `SceneSim`/`CombatState`, refactors `Replayer`, per-tick `TickOutcome` with restart/respawn flags; adapter in `src/bin/floor01_scene/replay_view.rs`; `Sim`/`Combat` stay sole owners; work order R0/R1 Build bullets replaced |
| **F-1** checked mode as one predicate | §3-B first two bullets, wording as recommended, plus ADR-0011 `replay_unverified` mapping |
| **F-2** type/output separation from verify | §3-B third bullet, §2 output line (PTR-Q14); test in §6 cross-build list |
| **F-3** companion data not in content hashes | §3-B fourth bullet; `eff`-bit tamper test in §6; U-1 |
| **F-4** divergence behaviour | §3-D (PTR-Q15) |
| **F-5** cross-build test technique; R0 needs no `src/bin/playtrace.rs` | §6 cross-build list; §5 and work order R0 ownership drop `src/bin/playtrace.rs`. Added: empty-fingerprint rejection in both modes. The `parse_verify_output` check is phrased as a Rust assertion on the line prefix, because `cargo test` cannot run #161's Python |
| **F-6** presentation settings not recorded | §2 settings source (PTR-Q13). Added `--cf02-seed`, `--demo-beat`, `--donut-biscuit`, `--manual`, `--lab-*` and `--capture-sequence` to the rejected list, since they also change sim state or the run mode; U-2 |
| **F-7** pads and HUD in replay | §4 item 1 "Inputs and HUD"; §2 playback controls; parse test in §6 |
| **F-8** stage acceptance table, R1 end-to-end check | §6 table and R1 smoke |
| **F-9** impossible two-deck evidence | §6 Bevy/evidence: recorded deck only, override rejected |
| **F-10** R1 scope cut | §4 item 5 (no streaming in R0/R1), restart moved to R2 (§2, §5), particle wall-clock → labelling only (§4 item 2), "held input edge state" deleted from the R0 list |

No finding was declined. FOLLOW-UP U-1…U-6 are listed in §7 for separate PRs.
