# ADR-0011: Automatic local gameplay session capture as first-class ML evidence

- **Status:** Accepted (owner requirement, 2026-11-20); defaults PT-Q01..PT-Q16 adopted under the design-defaults directive; PT-A, PT-B, PT-C and PT-D implemented (PT-D merged as #161); PT-E pending (status line refreshed 2026-11-28; decision text unchanged)
- **Date:** 2026-11-20
- **Scope:** Development and internal playtest gameplay with a live input source (solo Carl, solo Donut, local 2P, demo, lab). Scripted, headless and CI paths are out of the default scope (PT-Q12).
- **Owners:** game design / runtime / AUTOPLAY / companion AI.
- **Detailed specification:** [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md)
- **Local agent delivery:** [PLAYTRACE-01 work order](../07-agents/playtrace-01-workorder.md)
- **Related:** [COMBAT-LAB-01](../01-design/combat-lab-01-advanced-playtest.md), [ADR-0006](ADR-0006-agent-first.md), [ADR-0009](ADR-0009-companion-ai-authority.md)

## Context

The existing deterministic Rust simulator and `autoplay-episode/1` replay are valuable but do not automatically capture **actual human** gameplay across the live party scene, retries, AI companion, checkpoint boundaries and continuous multi-room play. This means human feedback cannot be reliably connected to what the player saw and pressed. An interesting video or a seeded final result is insufficient for training behavior cloning, offline RL or checking why game-feel changed.

## Decision

1. **Record by default before the first gameplay tick for every live-input `floor01_scene` run** (and `floor01_scene --scene combat_lab`, LAB-00-wire), independent of which level is being played. Scripted, headless, CI, capture and presim paths are off by default because they are already reproducible; `--record` forces recording on and `--no-record` turns it off with a visible line. "Dev/QA build" means every build from this repository while no distributable profile exists. One constant `RECORD_DEFAULT_ON` holds the default, and changing it for a distributed build needs a new ADR. Release/user-facing consent and sharing are separate and are not silently inherited.
2. Use an independent, append-only, versioned `playtrace-session/1` **continuous session** format with per-episode/attempt boundaries. Preserve the existing `autoplay-episode/1` and test goldens. Keep the gameplay simulation authoritative; the recorder only observes inputs, actor/party ownership, events and checkpoints, and never builds a second simulator. **PT-B makes no simulation changes.**
3. Record at `CombatState::step_party` (or `SceneSim::step_party` without combat) the per-hero routed inputs from `hero_inputs`, plus ordered lifecycle control events. Replay **re-derives AI inputs** from the recorded build; a different build is `replay_unverified`. Capture both slots (provenance and consistency check), effective inputs including the AI source, ordered authoritative effects, capability/seed/content/build fingerprints and reset/checkpoint metadata. Policy observations reuse **ObservationV2** (AUTOPLAY-02a, AP-Q27, proposed in open PR #136, not merged) computed once per hero; PLAYTRACE defines no competing observation type. Keep privileged simulator state **separate** from training observations. v1 is `snapshot_support=reset_only`; serializable snapshots are a future enhancement with their own work order, not a reason to discard raw data.
4. **No missing-tick success claims:** capture/integrity failure is visible and makes the affected data ineligible for verified replay/ML; ordinary gameplay remains operational. Provide recoverable partial files, deterministic fresh-process replay verification and an offline schema-versioned dataset export with leakage checks.
5. Local-only collection by default. No uploads, raw account/hardware identifiers, voice, chat, automatic video or personal telemetry collection. The manifest stores repo-relative paths only (no hostname, username, home path, `PadId` or vendor serial) and wall clock rounded to the day. The REC indicator is shown at launch and when P2 joins; a session with `participants = includes_other_tester` is not exported without an explicit per-session `--export-consented`. **Recordings and exports are never committed to the repository.** A participant chooses whether to export/share recordings outside the local machine.

Adopted defaults PT-Q01..PT-Q16, the open-question list and acceptance gates: [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md#7-adopted-defaults-and-open-questions).

## Alternatives considered

- **Video only:** rejected; visuals cannot faithfully recover command timing, actor ownership, unseen legal state or authoritative events.
- **Only input tape + seed:** useful for replay but fragile across binary/content changes and insufficient for offline imitation without a working historical simulator; rejected as the *sole* dataset layer.
- **Full state every tick:** unnecessary I/O/storage complexity and risks privileged-information leakage; use per-tick observations/transitions plus deterministic replay.
- **In-engine training/inference service:** rejected for this milestone; use the AUTOPLAY native core and external optional Python training tools.
- **Opt-in every internal test:** rejected; too many unrecorded playtests, reducing the value of rare human feedback. Public release requires a different consent policy.
- **Extend `autoplay-episode/2` (PR #136) instead of a new format:** rejected, because it is a single-episode, policy-driven artifact with no control events, no multi-episode session and no human slots. A version-aware bridge/exporter connects the two instead.
- **Store only the replay journal and derive observations offline by replay:** partly adopted. PT-B is journal-only; PT-C adds live observations as the fallback for a build that can no longer be replayed.

## Consequences and migration

The first PLAYTRACE implementation is **infrastructure**, not proof of intelligent AI or a tuned game. Its capture hook is attached to the real `floor01_scene` `fixed_step` before LAB-01's first human test, and gates calling a playtest "recorded" (it does not block the CF02-A/B owner game-feel playtest). AUTOPLAY-01, CF02 and COMPANION-AI remain owners of their respective logic; PLAYTRACE can import their stable interfaces but must not rewrite their behaviors. A recorder-disabled runtime must have identical authoritative simulation hashes. Replays without complete observation data may remain useful for diagnosis but are explicitly **not ML-ready**.

- **Disk use:** about 9 MB per minute uncompressed for two float32 ObservationV2 per tick (rough estimate); a warning above 5 GB, no automatic deletion (PT-Q15).
- **CI and scripted default-off:** CI scripts produce no recording directory (PT-Q12).
- **No new crates in v1:** no compression and no random-number crate; any addition needs a primary-source version check.
- **No sim changes in PT-B:** the accessor `SceneSim::last_effective_inputs()` arrives in PT-C and is unhashed.

No game release, game recordings or model weights are made distributable under the project's separate franchise-rights constraints merely by this ADR.

**Reversibility:** schema versions and offline export are replaceable with declared migrations; the default local recording policy can be changed by a new ADR; existing content-addressed session files are never silently upgraded or re-labelled as verified.
