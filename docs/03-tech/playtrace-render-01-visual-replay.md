# PLAYTRACE-RENDER-01 — Reconstruct and inspect recorded gameplay in Bevy

**Status:** design and implementation contract ONLY (2026-11-21). No visual playback CLI, timeline, or export exists on main. **Priority:** P1 tooling for the next combat/companion playtest; do not block normal game startup. **Owner feedback:** real CF02 fights exposed weak companion participation and visually promising goblin collisions. We need to inspect the *actual human actions and rendered consequences*, not just inspect a headless simulator.

## 1. Baseline and source of truth

Read [PLAYTRACE-01](playtrace-01-session-capture.md), [ADR-0011](../05-decisions/ADR-0011-gameplay-session-recording.md), [floor01-scene](floor01-scene.md), [COMBAT-LAB-01](../01-design/combat-lab-01-advanced-playtest.md), [companion spec](companion-ai-command-system.md), and [agent work order](../07-agents/playtrace-render-01-workorder.md).

- PLAYTRACE-A/B/C (merged PRs #143 and #156) write `manifest.json`, `control.jsonl`, and 600-tick `ticks-NNNNNN.jsonl` journals. `src/playtrace/codec.rs::TickRecord` contains exact per-slot/per-hero button+axis values, control sources, authoritative events, tick hashes and ObservationV2 where supported. The reset descriptor and control stream identify scene, party changes, restarts, pauses, checkpoint transitions and rule settings.
- `src/playtrace/replay.rs::verify_loaded` already reconstructs the real `SceneSim` / `CombatState` and compares the deterministic timeline. It is **headless**: it never calls Bevy rendering, animation, HUD, camera presentation or audio. The recorded `ph` (party hash) is not fully re-derived. Its preflight rejects changed source fingerprints.
- `src/bin/floor01_scene.rs` is the existing Bevy presenter with render-loop screenshot facilities (`--capture-sequence`) for **scripted** fixtures, NOT recordings from human input.
- PT-D (PR #161, currently open) is an **offline ML/data exporter**, not a visual playback engine. Do not edit `tools/playtrace/**`, PT-D schemas or training eligibility to implement visual playback.
- The current recorder stores **reset-only** state: no mid-run snapshots. Several content-hash coverage holes are documented in PLAYTRACE-01. Do not claim byte-identical audiovisual reconstruction just because a simulation hash matches.
- **Recent owner sessions are already recorded** on the older `0bffd264` build (per PT-D PR #161). Any new renderer build changes the coarse source fingerprint even when combat code is untouched. Supporting these pre-feature recordings is an explicit acceptance case; see §3.

## 2. Product contract

Start with a local, offline-only, native Bevy **replay viewer** of an existing Playtrace session; no video capture is needed at record time. Opening a session replays the original timeline from tick zero and draws Carl, Donut, goblins, terrain, death/gore, camera, HUD and other existing presentation layers **through the same scene presentation code as live gameplay**.

Required initial CLI (final exact names may follow local argument conventions, but record any adopted change in the PR):

    cargo run --locked --release --bin floor01_scene -- --replay-session <SESSION_ID> --scale 3
    cargo run --locked --release --bin floor01_scene -- --replay-session <SESSION_ID> --replay-compatibility checked

The first command means strict same-build replay. The second is an **explicitly opted-in** cross-build visual reconstruction mode for pre-feature recordings; it never means `playtrace verify` passed under a mismatched source fingerprint.

**Playback controls** (developer viewer, not game bindings): pause/resume; frame/tick step; restart; show episode boundary and current `t` / `et` / seed / build / human-vs-AI ownership. Minimal keyboard shortcuts are acceptable; keep all controller gameplay bindings unchanged. Normal player/pad events must not alter the reconstructed world or join P2. Replay viewer does not start another Playtrace recording.

**Next UX**: jump to a tick or event via replay-to-target from reset (no snapshot assumption), navigate between attempts/deaths, event bookmarks, speed 0.5x/1x/2x and an optional free debug camera. Viewer can compare an AI decision or contact with recorded `ObservationV2` and real event data without interpreting the debug-text `EventRecord.d` as a stable typed event schema. For now expose verified event labels and relevant in-simulation state from the matching simulator version; richer typed diagnostics come via a separate schema PR if needed.

**Export**: deterministic per-simulation-tick rendered PNG sequence at 60 Hz with a sidecar manifest (session/tick/episode/build/rules, presentation settings, verification class, checksums, FPS). An **optional external** FFmpeg wrapper can encode MP4 or GIF from that sequence; no system FFmpeg/runtime dependency in the Rust game and no claim of bit-identical audio/video. Exports must be opt-in and written outside git. Prefer an integration of existing `--capture-sequence` / frame capture infrastructure rather than a second renderer.

## 3. Replay trust classes — the critical compatibility decision

**A. Strict / same-source** (new recordings made with a renderer-enabled build): retain the exact existing `playtrace verify` contract, including schema, source fingerprint, rules, reset/content checks, every tick's `pre` / `post` / `eh`, effective hero inputs and event ordering. Fail closed on missing ticks, unsupported source, corrupted stream, unverifiable reset, and mismatched content. Report the party-hash limitation truthfully until fixed.

**B. Checked / cross-source** (older human CF02 sessions): explicitly allow source fingerprint mismatch **only inside a viewer-specific compatibility path**. Rebuild using the current Rust simulation and compare each recorded tick's reset, effective input, state hashes, ordered events, event hashes and lifecycle; detect/abort on the first divergence, with tick and expected/actual evidence. Reject incompatible schemas, rule flags, geometry/assets/content, missing data and unhandled control transitions. The built-in `playtrace verify` **remains unchanged and strict**; PT-D's export/training gates must never accept checked-mode replay as strict verification. Display an always-visible label such as `CHECKED CROSS-BUILD (NOT ORIGINAL-BUILD VERIFIED)` and record that status in exported sidecars. An original-build `playtrace verify` PASS, if available, is useful provenance but must not be faked. If the code is not compatible, state precisely that the old recording cannot currently be rendered and retain the data.

Never turn off divergence checks, silently fall back to autoplay, or rewrite manifests/hashes. Specifically do not use the existing party `ph` as proof of a verified party-state check until its derivation is implemented. The stronger path is a future *simulation fingerprint* decoupled from presentation-only source changes, designed separately and never assumed retroactively.

**Visual fidelity level:** gameplay-authoritative state/event equivalence is the hard gate; pixel-exact presentation is only promised when verified independently with matching assets, presentation settings, deterministic event-driven animation/VFX and a known renderer environment. Replayed camera state should follow the same simulation and presentation rules. Presentation-only randomness needs event/session seeding or a recorded presentation stream; otherwise call the frame an approximate reconstruction. At variable speed, audio may be muted rather than pitch-shifted. Do not confuse a tick pause with a recorded wall-clock pause.

## 4. Implementation architecture

One authoritative replay driver, no second combat implementation:

1. Extract a small **Bevy-free replay cursor/stepper** from the existing verifier where feasible: load and validate a session, build its reset once, apply `ControlEvent` at the correct `at_t` boundary, step the existing sim using recorded **routed per-hero input**, rederive AI effective inputs, and emit per-tick state/events plus an explicit divergence verdict. Share *the same transition/control application* with strict `verify_loaded`; avoid copying a large second `match reset.source` or competing combat rules into the Bevy binary. Work with PT-D owner if changes overlap.
2. Add a separate **replay drive mode** in `floor01_scene` at its current fixed-step/input seam. Disable real gameplay input, auto-recording and idle autopilots. Feed one recorded tick per simulation step, apply its party/scene controls exactly once, and route the resulting real `SceneSim` / `CombatState` into existing render sync (camera, actors, animations, particles, HUD). Presentation must not mutate gameplay/RNG/recorded hashes.
3. Keep authoritative simulation tick clock separate from render FPS, animation frames, and wall time. At 1x advance 60 recorded ticks per simulated second; in deterministic frame export, advance exactly one tick per output frame. Pausing freezes simulation and gameplay-driven presentation. Any arrival of a second real controller is viewer input only, never a party join.
4. Handle sessions with retries, automatic respawn, checkpoints, solo swaps, AI Carl or AI Donut, local 2P join/quit, source changes and session-end explicitly. Do not initialize a room directly to an arbitrary mid-session position without reproducing prior transitions. First eligible source: human `--cf02` session; then `--demo` and fixtures; gate story/combat-lab support by concrete successful reset reconstruction (lab reset remains a known PT-C review risk).
5. Put replay debug presentation in a separate overlay resource/module: tick, time, actor authority, pause/speed, verification class, selected event and clear cause-of-impact/target info. Existing HUD may remain visible but gameplay controls must not leak into it. Do not interpret `EventRecord.d` text to derive authoritative damage or state.
6. Avoid unbounded memory use: stream the journal/chunks or bound buffered ticks; cap event/diagnostic views; no gigantic 13k-tick retained Bevy entity history. Seeking v1 is replay-from-reset, and must display progress if it takes time; snapshots/indexing are a separate later optimisation.

## 5. Work packages and dependencies

| ID | Deliverable | Depends on | Done when |
| --- | --- | --- | --- |
| **PTR-R0** | Shared deterministic replay cursor, strict and checked compatibility policies, tests | PT-C #156 | Handles reset/controls/ticks identically to verifier, rejects divergence; old session compatibility path works if sim matches |
| **PTR-R1** | Bevy render integration, CLI, play/pause/step, default camera/HUD/animations | R0 | Real human `--cf02` session is visually replayable on Linux; no live input or new recording; matched events/hashes |
| **PTR-R2** | Seek/restart, bookmarks, speed controls, diagnostics overlay and per-hero event inspection | R1 | Can find a Donut inactivity window and a goblin-to-goblin hit without manual video scrubbing |
| **PTR-R3** | 60 fps offline PNG export with frame/tick metadata, optional FFmpeg helper | R1; R2 if timeline-range export requested | Same input yields same frame sequence in same build/environment; export is opt-in |
| **PTR-R4** | Controller/owner acceptance, audio and presentation parity, performance and regression evidence | R1-R3 | Verified replay of owner session, recorded known limitations, no production gameplay regressions |

Separate PRs are preferred for R0+R1 vs R2 vs R3; R0 alone is not a useful product milestone. An integrator may group R0 and R1 for an end-to-end usable MVP after independent review. **No hard dependency on PT-D #161, ML training, companion redesign or LAB-02.** Coordinate any `src/playtrace/**` edit with PT-D until #161 merges. Keep gameplay mechanics and live party rules unchanged.

## 6. Acceptance and tests

- **Unit / headless:** synthetic fixed input recordings (one Carl, solo swap, AI Donut, human Donut + AI Carl, real party drive with P2 join, death/retry, scene break, pause event) reproduce source events and every supported hash. Repeated start/stop, missing chunk, altered action bit, event edit, different seed, stale source, incompatible content, unsupported mode all fail with actionable messages.
- **Cross-build:** synthetic recording made with a known earlier unmodified build is replayed on the modified renderer-enabled binary only under `checked` mode, proving every supported sim hash/event parity. A deliberate change to combat/seed/room rejects it. Strict mode still fails on fingerprint mismatch; strict exporter checks still fail. Do not commit human Playtrace data to tests or Git.
- **Bevy/evidence:** draw real CF02 640×360 and 640×400 frames and a short 1x GIF at the same 3 selected ticks (kick, body contact, Donut support or documented inactivity) and compare episode/tick, positions, camera bounds, sprites, environmental state and event counts with headless replay. Test frame export reproducibility in an environment with pinned software rendering, but distinguish identical in-environment pixels from cross-hardware guarantees.
- **Visual/interaction:** start/pause/step/restart, seek across a respawn, two-pad session rendering without live input stealing control, one end-of-session clean exit; scene and HUD correctly reinitialize at episode transition. No extra combat event, RNG advance, companion move, camera teleport or replay writer output.
- **Performance:** 10–15k tick owner-like session: first-frame latency, seek-from-start wall time, sustained playback frame-time p50/p95/p99, memory high watermark, export wall time and total disk bytes. Measure on named hardware; do not invent target frame numbers.
- **Regression:** `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `bash scripts/ci_floor01_combat.sh headless`, `python3 scripts/check_docs.py`; all existing goldens unchanged. Capture source/action version matrix and evidence in PR bodies.
- **Owner acceptance:** one real human `--cf02` recording showing the same fighting sequence the owner remembers, including Donut's inactivity and a body collision; clearly differentiate what is verified, not verified and visually approximate. A second 2P physical-pad recording remains NOT RUN unless actually performed.

## 7. Non-goals and rights

No video captured during live gameplay, no upload/telemetry service, no ML inference, no automatic blame of companion decisions, no screenshot generation for copyrighted third-party assets, no change to combat damage or companion AI, no undocumented command rebinding, no compulsory periodic snapshots or rewrite of the PLAYTRACE session schema. Recordings and exports are private local files; **never** commit identifiable session data, user home paths or full personal recordings. The public fan-concept assets remain under the repo's rights policy; a video export is not permission to distribute a game/trailer.

**Decisions for agents (adopted defaults pending evidence):** PTR-Q01: replay source of truth = existing sim + recorded routed inputs; Q02: default strict; explicit checked compatibility for pre-feature builds; Q03: PNG first, external FFmpeg optional; Q04: 60 fps one tick/output frame; Q05: seek from reset until snapshot work; Q06: pause/playback debug controls in viewer only; Q07: metadata always declares trust class and render settings; Q08: no networking or recorder loop; Q09: no change to PT-D eligibility; Q10: MVP targets CF02 human sessions first and adds other modes only as replay reconstruction is proven.
