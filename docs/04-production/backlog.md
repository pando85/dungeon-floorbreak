# Agent-ready prioritized backlog

The repository includes design docs, a Bevy WORK-001 runtime spike, approved mood references, and WORK-008 editable art/export fixtures. Status of each item depends on its own CI and review evidence; see [work order protocol](../07-agents/workorders.md).

| ID | Priority | Stage | Deliverable | Acceptance evidence |
| --- | --- | --- | --- | --- |
| WORK-021 | P0 | Prototype / merged PR #21 | Combat-first, right-scrolling Bevy Floor 1 greybox; melee, goblins, bombs, destructible timber and forced steel climb | Automated CI, fixed idle-safe spawn regression, real Bevy rendered evidence; human gamepad testing deferred to integrated scene; see [WORK-021 brief](linear-combat-greybox.md) |
| WORK-022 | P0 | Art / merged PR #22 | Native-scale Bevy Character Playground (Carl 52 px, Donut 40 px, Mongo 76 px) with atlas exports and corrected mirrored foot-pivot alignment | Automated CI and real Bevy 640×360/640×400 screenshots; human gamepad/visual signoff deferred to integrated scene; see [playground](../03-tech/character-playground.md) |
| ENV-01 | P0 | Art / owner-approved art on 2026-11-16 (round 4, PR #26); in-engine lighting re-tune pending in COMP-01 | Floor 1 catacomb environment kit in the A1 look: editable pixel-grid/.ase sources for deep-floor collision tiles (non-collidable `deck_back` + walkable `deck_front` with a walking line), steps/edges, ceiling/cornice, end walls; five warm brick height bands + pale base course; baked torch pools and grime; thick A1 arch ring, piers, threshold, pilaster; far/mid gallery parallax strips; copper/brass pipe set, valve, collars, timber door, unlit sconce, lichen; 4-frame torch; per-atlas palettes (max 63) from the 112-colour `floor01-catacomb-v2` kit palette; `env-kit-v2` contract (walk line + colliders, parallax, arch windows, light emitters) and 128x25 `env-kit-layout-v2` sample; [write-up](../02-art/floor-01-catacomb-kit.md), [evidence](../evidence/floor01-catacomb-kit/README.md) | Deterministic export + `scripts/env_kit.py` v2 validation and 97 tests in CI; `scripts/env_kit_light.py` lit renders (640x360, 1920x1080 x3, Deck 640x400 x2, 2048x400 overview), side-by-side vs A1, Carl/Donut readability measurements. **Owner-approved 2026-11-16**; Bevy rendering and lighting are COMP-01 |
| COMP-01 | P0 | Integration / merged (PR #29), awaiting human playtest | First integrated Floor 1 corridor scene: the ENV-01 kit layout rendered at 640×360 / 640×400 with the approved Carl 52 px and Donut 40 px atlases (Donut a passive visual companion), deep-floor walking-line collision from the contract colliders (0 px foot sink), deterministic scroll camera, parallax strips and tick-driven torch frames. **Pixel layers only; lighting, bloom and haze are COMP-02.** Separate `floor01_scene` binary; the greybox and Character Playground are unchanged except the shared `step_hero` extraction (golden-trace tested) | Rust unit tests, `scripts/check_floor01_scene_contract.py` plus negative tests, headless state-line equality with the render, five Xvfb screenshots in CI (`the-crawl-floor01-scene`); [scene doc](../03-tech/floor01-scene.md), [evidence and manual controller checklist](../evidence/floor01-scene/README.md). Human game-feel, readability and controller review pending; no performance data |
| COMP-02 | P0 | Rendering / merged (PR #30), awaiting human review | A1 lighting pass in Bevy for the Floor 1 corridor scene, stacked on COMP-01: pixel layers at virtual resolution, light maths at output resolution (cold ambient per group, warm torch pools, floor pools on the walking line, contact shadows, per-layer haze, additive glow, flame-only bloom); 6 slot targets plus emissive, blur and composite stages as `Material2d` quads; lights derived from the kit contract; `--no-lighting`, `--reference-frame`, `--perf-frames`; [scene doc](../03-tech/floor01-scene.md#lighting-comp-02), [ADR-0007 note](../05-decisions/ADR-0007-rendering-resolution-lighting.md) | Rust unit tests (light derivation, y flip, floor pools, flicker, GPU packing, profile validation, gather leaves the sim hash untouched, load-time light budget, shader/Rust layout via naga), CI renders at 1920x1080 / 1280x720 / Deck / advanced / `--no-lighting` / reference / collision, live-stepped lit and `--no-lighting` runs must end in the same state as the headless sim, Bevy-vs-Pillow comparison on the ENV-01 preview frame (mean dE76 0.55, gated in CI; flame halos differ most), frame intervals on an RTX 4060 Ti (release 1.28 ms lit vs 0.71 ms unlit at 1080p). **Human review of the look and feel pending**; no Steam Deck measurement |
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
| ART-LOOKDEV-01 | P0 | Art spike / merged (PR #27) | Floor 1 resolution and lighting look-dev: one composition as A0 (640×360 baked), A1 (640×360 + full-resolution lighting pass) and B1 (960×540 redraw + lighting); [write-up](../02-art/lookdev-floor01-resolution-spike.md), [evidence](../evidence/lookdev-floor01-resolution-spike/README.md) | **Owner chose A1 on 2026-11-16** (B1 and the h78/h60 cast redraws rejected), recorded in [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md). A1 is the target frame for the ENV-01 rework; COMP-01 implements the lighting pass |
| WORK-007/008 follow-up | P0 | Decision | Floor 1 rendering target: 640×360 virtual resolution with a full-resolution lighting pass, parallax layers and a deep floor; [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md), look-dev spike PR #27 | **Owner decided 2026-11-16** (A1 chosen, B1 rejected). Docs only: ENV-01 rework (PR #26) and COMP-01 engine lighting follow; Bevy 0.20 implementation not yet verified |
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

## Floor 01 art atlas v1 — area concepts (2026-11-16)

- **Concept direction selected:** [Floor 01 ten-area atlas](../02-art/floor-01-art-atlas-v1.md) with a shared A/B/C catacomb substrate, distinct territories and special rooms. Book-derived attributes are distinguished from artistic interpretation.
- **Agent sequence:** [area-kit plan](../07-agents/floor-01-area-kits-plan.md), beginning **after** the integrated Bevy corridor [COMP-01 PR #29](https://github.com/pando85/dungeon-floorbreak/pull/29) has actual evidence and a human visual/game-feel gate.
- **Concept PNGs committed and verified:** [ten image references (overview + areas 02–10)](../../assets/concept/floor-01/atlas-v1/README.md) and a [SHA-256 manifest](../../assets/concept/floor-01/atlas-v1/manifest.json) are now in the PR; all ten Git blob identities match the original local reference bytes. Area 01 remains in the previously merged [opening concepts](../../assets/concept/floor-01/opening/README.md).
- **Known incompleteness:** Gym and Ball of Swine use **provisional cropped atlas panels** rather than separate full-resolution concept art; neither arena geometry nor sprite output is approved. No new game mechanics are added by this doc PR.

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

## WORK-022 — Character Playground (PR #22)

**Scope:** Independent Rust + Bevy 0.20.1 cast-inspection executable; 52 px Carl, 40 px Donut and 76 px adult Mongo at original virtual-pixel scale with four-frame `idle`, pivot-aligned fixed-step movement/jump, keyboard/gamepad selection, and 640×360 / 640×400 camera baselines.

**Evidence target:** `.github/workflows/character-playground.yml` exports existing editable masters, checks generated atlas contracts, runs Rust tests plus a headless smoke, and uploads two true Bevy-rendered Linux screenshots. Follow `docs/03-tech/character-playground.md`.

**Status:** Merged via PR #22 on 2026-11-16. All four workflows passed on the final PR head; real Bevy 640×360 and 640×400 screenshots were reviewed technically. Owner explicitly deferred hands-on gamepad/visual acceptance to the **integrated Floor 1 scene**. Native sizes are approved; locomotion/jump animation, overlapping silhouettes and mounted Donut remain future work. The playground binary is independent of the combat greybox until a separate integration PR.
