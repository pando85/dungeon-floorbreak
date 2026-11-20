# ADR-0011 — Automatic local gameplay session capture as first-class ML evidence

**Status:** Proposed for adoption as project design policy (2026-11-20).
**Owners:** THE CRAWL game design / runtime / AUTOPLAY / companion AI.
**Scope:** all development and internal playtest gameplay (solo Carl, solo Donut, local 2P, bot, demo, lab).

## Context

The existing deterministic Rust simulator and `autoplay-episode/1` replay are valuable but do not automatically capture **actual human** gameplay across the live party scene, retries, AI companion, checkpoint boundaries and continuous multi-room play. This means human feedback cannot be reliably connected to what the player saw and pressed. An interesting video or a seeded final result is insufficient for training behavior cloning, offline RL or checking why game-feel changed.

## Decision

1. **Record by default before the first gameplay tick** in local development and QA/playtest builds, independent of which level is being played; explicit visible `--no-record` is available. Release/user-facing consent and sharing are separate, not silently inherited.
2. Use an independent, append-only, versioned `playtrace-session/1` **continuous session** format with per-episode/attempt boundaries. Preserve the existing `autoplay-episode/1` and test goldens. Keep the gameplay simulation authoritative; the recorder only observes inputs, actor/party ownership, legal observations, events and checkpoints. Never build a second simulator.
3. Capture full actual per-tick inputs, both slots and both hero effective inputs including the AI source, pre-action policy observations, post-action observations, ordered authoritative effects, capability/seed/content/build fingerprints and reset/checkpoint metadata. Keep privileged simulator state **separate** from training observations. Periodic full-state snapshots are a future-enabling enhancement, not a reason to discard raw data in their absence.
4. **No missing-tick success claims:** capture/integrity failure is visible and makes the affected data ineligible for verified replay/ML; ordinary gameplay remains operational. Provide recoverable partial files, deterministic fresh-process replay verification and an offline schema-versioned dataset export with leakage checks.
5. Local-only collection by default. No uploads, raw account/hardware identifiers, voice, chat, automatic video or personal telemetry collection. A participant chooses whether to export/share recordings outside the local machine.

Detailed adopted defaults PT-Q01..PT-Q10 and acceptance gates: [PLAYTRACE-01](../03-tech/playtrace-01-session-capture.md).

## Alternatives considered

- **Video only:** rejected; visuals cannot faithfully recover command timing, actor ownership, unseen legal state or authoritative events.
- **Only input tape + seed:** useful for replay but fragile across binary/content changes and insufficient for offline imitation without a working historical simulator; rejected as the *sole* dataset layer.
- **Full state every tick:** unnecessary I/O/storage complexity and risks privileged-information leakage; use per-tick observations/transitions plus periodic world snapshots and deterministic replay.
- **In-engine training/inference service:** rejected for this milestone; use the AUTOPLAY native core and external optional Python training tools.
- **Opt-in every internal test:** rejected; too many unrecorded playtests, reducing the value of rare human feedback. Public release requires a different consent policy.

## Consequences and migration

The first PLAYTRACE implementation is **infrastructure**, not proof of intelligent AI or a tuned game. Its capture hook must be attached to the real `floor01_scene` fixed-step dispatch before LAB-01 testing. AUTOPLAY-01, CF02 and COMPANION-AI remain owners of their respective logic; PLAYTRACE can import their stable interfaces but must not rewrite their behaviors. A recorder-disabled runtime must have identical authoritative simulation hashes. Replays without complete observation data may remain useful for diagnosis but are explicitly **not ML-ready**.

No game release, game recordings or model weights are made distributable under the project's separate franchise-rights constraints merely by this ADR.

**Reversibility:** schema versions and offline export are replaceable with declared migrations; default local recording policy can be changed by a new ADR; existing content-addressed session files are never silently upgraded or re-labelled as verified.
