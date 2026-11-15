# Agent-ready prioritized backlog

The repository currently ships design/specification documents only. Status: `proposed` until a PR and actual test evidence say otherwise. See [work order protocol](../07-agents/workorders.md).

| ID | Priority | Stage | Deliverable | Acceptance evidence |
| --- | --- | --- | --- | --- |
| WORK-001 | P0 | Spike | Rust Bevy app, one pixel camera and normalized gamepad action layer | App starts on Linux; test action adapter; capture screenshot |
| WORK-002 | P0 | Spike | Single character with jump buffering and coyote-time tuning values | Replayed jump fixtures, controller feel note |
| WORK-003 | P0 | Spike | Minimal material grid + bomb damage and tile visualization | Same event/seed ⇒ same grid hash; wall breaks |
| WORK-004 | P0 | Spike | Dirty-chunk collider rebuild with seams and level reachability checks | Cross-boundary tests; no remaining ghost colliders |
| WORK-005 | P0 | Prototype | Second controllable hero and two-player device ownership | Two pads, no cross-talk; reconnect scenario |
| WORK-006 | P0 | Prototype | Solo swap vs follower AI experiment | Short recording of both; decision record |
| WORK-007 | P0 | Art | Canonical dark pixel-art style bible and demo palette | Human signoff at 1× and 2× |
| WORK-008 | P0 | Art | Aseprite Lua export + spritesheet/pivot/tag validation | CI artifact from source; consistent alpha/pivot |
| WORK-009 | P1 | Slice | Carl/demolitionist bomb + one melee attack | Deterministic damage and cooldown tests |
| WORK-010 | P1 | Slice | Donut/caster spell controlling a chaser | Cooperative combo recording and unit tests |
| WORK-011 | P1 | Slice | One LDtk authored room with two paths and critical geometry | Schema check; path invariants under blasts |
| WORK-012 | P1 | Slice | Two enemy states/archetypes and navigation recovery | Terrain-modification scenario, no permanent stuck AI |
| WORK-013 | P1 | Slice | Destructible props and bounded explosive chain | Event cap tests; visual feedback review |
| WORK-014 | P1 | Slice | Checkpoint, death/retry, fully controller-driven menu | Gamepad-only 3-minute test session |
| WORK-015 | P1 | Slice | AI visual regression pipeline and contact sheets | Artifacts attached to PRs; manual review record |
| WORK-016 | P1 | Slice | Steam Deck-class frame-time instrumentation | Profile captured; no unqualified fps claims |
| WORK-017 | P2 | After slice | Mongo/charger prototype | Character distinctness + non-softlocking charge |
| WORK-018 | P2 | After slice | Authored small structural collapse | Weak-support graph, VFX and invariant tests |
| WORK-019 | P2 | After slice | Steamworks and Windows portability study | Rights prerequisite acknowledged; build/smoke test |
| WORK-020 | P2 | Gate | Legal/IP release decision and third-party asset inventory | Permission or original-IP alternative documented |

## Visual target review (WORK-007)

[VT-001 — Inspection Corridor](../02-art/visual-target-inspection-corridor.md) is
a **static art-direction candidate** for human review, not a playable build or
approved production sprite. Its image remains in the private project
conversation pending rights/provenance review. Before accepting WORK-007, obtain
owner approval of art tone plus a native-resolution Aseprite/Bevy readability
sample; the concept illustration alone does **not** meet the acceptance gate.

## First agent work order (WORK-001)

**Goal:** start a clean, reproducible Rust 2024 Bevy 0.20.x Linux binary with a 2D pixel camera, empty test level, a gamepad-neutral action map and one headless action test. **No character likenesses.**

**Scope:** a small workspace or crate, lockfile, README build/run command, one input module and one CLI/headless test. UI must be controllable with at least one gamepad input adapter or a testable synthetic action stream. Do not integrate Steam SDK, Avian or LDtk until their role is needed.

**Expected acceptance:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run`; include actual test logs and a screenshot from a Linux session. Report skipped GPU/hardware test honestly.

**Notes:** Pin the specific 0.20 patch release after checking current crate metadata; do not blindly copy API examples from older Bevy versions. Obtain human acceptance of pixel scaling before WORK-003.
