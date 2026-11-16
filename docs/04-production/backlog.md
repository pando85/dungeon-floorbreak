# Agent-ready prioritized backlog

The repository includes design docs, a Bevy WORK-001 runtime spike, approved mood references, and WORK-008 editable art/export fixtures. Status of each item depends on its own CI and review evidence; see [work order protocol](../07-agents/workorders.md).

| ID | Priority | Stage | Deliverable | Acceptance evidence |
| --- | --- | --- | --- | --- |
| WORK-021 | P0 | Prototype / in review | Combat-first, right-scrolling Bevy Floor 1 greybox; melee, goblins, bombs, destructible timber and forced steel climb | Unit + headless combat smoke + Linux rendered screenshot; human playtest; see [WORK-021 brief](linear-combat-greybox.md) |
| WORK-001 | P0 | Spike | Rust Bevy app, one pixel camera and normalized gamepad action layer | App starts on Linux; test action adapter; capture screenshot |
| WORK-002 | P0 | Spike | Single character with jump buffering and coyote-time tuning values | Replayed jump fixtures, controller feel note |
| WORK-003 | P0 | Spike | Minimal material grid + bomb damage and tile visualization | Same event/seed ⇒ same grid hash; wall breaks |
| WORK-004 | P0 | Spike | Dirty-chunk collider rebuild with seams and level reachability checks | Cross-boundary tests; no remaining ghost colliders |
| WORK-005 | P0 | Prototype | Second controllable hero and two-player device ownership | Two pads, no cross-talk; reconnect scenario |
| WORK-006 | P0 | Prototype | Solo swap vs follower AI experiment | Short recording of both; decision record |
| WORK-007 | P0 | Art | Canonical dark pixel-art style bible and demo palette | Human signoff at 1× and 2× |
| WORK-008 | P0 | Art | Aseprite Lua export + spritesheet/pivot/tag validation | CI artifact from source; consistent alpha/pivot |
| WORK-007/008 follow-up | P0 | Art | Donut floor 1 native scale study (32/40/48 px) and 40 px idle prototype; [write-up](../02-art/donut-scale-study.md), [evidence](../evidence/donut-floor1-scale-study/README.md) | **Human approved 40 px on 2026-11-16** ([decision](../02-art/character-scale-approval.md)); existing 4-frame idle is a study prototype. No Bevy check or run-cycle acceptance yet |
| WORK-007/008 follow-up | P0 | Art | Carl Book 1 native scale study (48/52/56 px) and 52 px idle prototype; [write-up](../02-art/carl-scale-study.md), [evidence](../evidence/carl-book1-scale-study/README.md) | **Human approved 52 px on 2026-11-16** ([decision](../02-art/character-scale-approval.md)); idle prototype only. Open canon questions: sleeves, Crocs, smiley pin. No 6-frame run or Bevy check yet |
| WORK-007/008 follow-up | P0 | Art | Mongo adult native scale study (64/76/88 px) and 76 px idle prototype; [write-up](../02-art/mongo-scale-study.md), [evidence](../evidence/mongo-adult-scale-study/README.md) | **Human approved adult 76 px on 2026-11-16** ([decision](../02-art/character-scale-approval.md)); idle prototype only. Open canon questions: toe/sickle claw, teeth/mouth. No 6-frame run, mounted Donut or Bevy check yet |
| WORK-009 | P1 | Slice | Carl/demolitionist bomb + one melee attack | Deterministic damage and cooldown tests |
| WORK-010 | P1 | Slice | Donut/caster spell controlling a chaser | Cooperative combo recording and unit tests |
| WORK-011 | P1 | Slice | One LDtk authored room with two paths and critical geometry | Schema check; path invariants under blasts |
| WORK-012 | P1 | Slice | Two enemy states/archetypes and navigation recovery | Terrain-modification scenario, no permanent stuck AI |
| WORK-013 | P1 | Slice | Destructible props and bounded explosive chain | Event cap tests; visual feedback review |
| WORK-014 | P1 | Slice | Checkpoint, death/retry, fully controller-driven menu | Gamepad-only 3-minute test session |
| WORK-015 | P1 | Slice | AI visual regression pipeline and contact sheets | Artifacts attached to PRs; manual review record |
| WORK-016 | P1 | Slice | Steam Deck-class frame-time instrumentation | Profile captured; no unqualified fps claims |
| WORK-017 | P2 | After slice | Mongo/charger prototype | Character distinctness + non-softlocking charge; use the approved [Mongo art bible](../02-art/mongo-character-bible.md), source PNGs uploaded and hash-verified; adult **76 px scale approved**, with native gameplay/animation acceptance still pending, see the row above |
| WORK-018 | P2 | After slice | Authored small structural collapse | Weak-support graph, VFX and invariant tests |
| WORK-019 | P2 | After slice | Steamworks and Windows portability study | Rights prerequisite acknowledged; build/smoke test |
| WORK-020 | P2 | Gate | Legal/IP release decision and third-party asset inventory | Permission or original-IP alternative documented |

## New art-direction handoff — Floor 01 opening (2026-11-16)

- **Selected:** [opening catacomb + goblin / Murder Dozer concept direction](../02-art/floor-01-opening-catacomb-goblins.md). This locks the A/B/C environment grammar, gradual immersion, visually distinct goblins, and a pursuit-first Dozer encounter. It is **concept acceptance only**.
- **Deferred:** [local-agent implementation handoff](../07-agents/floor-01-opening-encounter-handoff.md). Start environment/sprite/chase work only after the ongoing Bevy Floor 1 greybox PR #21 and Character Playground PR #22 are reviewed/merged and the real 52/40 px character atlases can be composed.
- **Committed reference art:** [clean catacomb, three goblins and Murder Dozer](../../assets/concept/floor-01/opening/README.md) as **three PNG concept references**, with hashes and provenance in the [manifest](../../assets/concept/floor-01/opening/manifest.json). Git blobs were checked against the original review files; not native production sprites.
- **Not delivered:** editable terrain/enemy/Dozer masters, actual Bevy scene, chase logic, canon-accurate goblin pineapple-clubs, or player testing. The full chase illustration with incorrect Carl/Donut remains excluded.

## Visual target review (WORK-007)

[VT-001 — Inspection Corridor](../02-art/visual-target-inspection-corridor.md) is
**approved for overall visual mood and composition** by the owner (2026-11-15),
**not** for its Carl depiction, canonical character appearance, gameplay-scale
pixel art, playable level or final production assets. The illustrated Carl is
explicitly *not* the already-approved Carl; preserve the character bible. The
image remains in the private project conversation; publication is covered by
`ART-PUBLICATION-ALL-V1` but it still needs a provenance/checksum record. WORK-007 is still **in progress**: accept only after native-resolution
Aseprite/Bevy readability samples, source/provenance checks and human review.
The concept illustration alone does **not** meet its acceptance gate.

## First agent work order (WORK-001)

**Goal:** start a clean, reproducible Rust 2024 Bevy 0.20.x Linux binary with a 2D pixel camera, empty test level, a gamepad-neutral action map and one headless action test. **No character likenesses.**

**Scope:** a small workspace or crate, lockfile, README build/run command, one input module and one CLI/headless test. UI must be controllable with at least one gamepad input adapter or a testable synthetic action stream. Do not integrate Steam SDK, Avian or LDtk until their role is needed.

**Expected acceptance:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run`; include actual test logs and a screenshot from a Linux session. Report skipped GPU/hardware test honestly.

**Notes:** Pin the specific 0.20 patch release after checking current crate metadata; do not blindly copy API examples from older Bevy versions. Obtain human acceptance of pixel scaling before WORK-003.

## WORK-008 execution state

The Aseprite-compatible art pipeline is implemented by **PR #13**, with three
editable original-IP-neutral .ase source fixtures and LibreSprite v1.3 in CI.
The export, metadata checks, nine negative tests and byte-identical repeat
export **passed GitHub Actions**; the 640×360 and 2× contact sheets were
uploaded as CI evidence. This satisfies the **WORK-008 tooling spike**, but
not WORK-007's art quality requirement. A human review of actual native-scale
art is still required before a visual style can be labeled production-ready.
See [asset-pipeline.md](../02-art/asset-pipeline.md).

## WORK-021 — Character Playground (PR #22)

**Scope:** Independent Rust + Bevy 0.20.1 cast-inspection executable; 52 px Carl, 40 px Donut and 76 px adult Mongo at original virtual-pixel scale with four-frame `idle`, pivot-aligned fixed-step movement/jump, keyboard/gamepad selection, and 640×360 / 640×400 camera baselines.

**Evidence target:** `.github/workflows/character-playground.yml` exports existing editable masters, checks generated atlas contracts, runs Rust tests plus a headless smoke, and uploads two true Bevy-rendered Linux screenshots. Follow `docs/03-tech/character-playground.md`.

**Status:** Implementation proposed in draft PR #22; CI results and owner visual/game-feel signoff pending. Native sizes are approved, but locomotion/jump animation, overlapping silhouettes and mounted Donut are not. Do not merge this work into the Floor 1 combat greybox as part of its PR.
