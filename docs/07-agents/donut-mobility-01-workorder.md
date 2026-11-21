# DONUT-MOBILITY-01 — Implementation work order

**Status:** specification only; nothing is implemented. **Priority:** P1, owner request 2026-11-21. It lands **before or alongside CA2-A** of COMPANION-AI-02 (section "Order").
**Authority:** [design spec](../03-tech/donut-mobility-01.md). The DM-Q values there are adopted defaults: do not re-decide them. A design problem found during execution goes back to the top tier.
**Read first:** [AGENTS.md](../../AGENTS.md), [orchestration](agent-orchestration.md) (standard brief, speed mode), the spec sections each stage names.

## Routing

| Step | Tier | Notes |
| --- | --- | --- |
| Design (done: the spec) | Top tier | No further top-tier design until the owner's game-feel review, except design defects found in review |
| DM-A, DM-B, DM-C execution | Medium tier | Code, tests, fixtures, goldens, evidence, docs |
| Review of each PR | Independent top-tier agent | Against the spec and section 7 of it. Speed mode: a medium-tier fix check suffices unless the review found a design or logic defect |
| DM-ART (later) | Per the [pixel-art pipeline](pixel-art-pipeline.md#routing-by-difficulty) | Only after the owner picks an evasion mode |

Every stage:

- keeps the gate: Legacy everywhere except the `--cf02` profile and the arena tunings;
- keeps every golden byte-identical except those spec section 5 lists;
- runs `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked --all-targets`;
- leaves `docs/04-production/backlog.md` alone: speed mode, the orchestrator updates it after merge, and the PR body carries the status.

## Stage DM-A — Agile motion (code PR 1)

**Goal:** under `MotionProfile::Agile`, Donut runs 207 px/s and jumps 485 px/s, human or AI. Everything else is byte-identical.

**Files owned:**

- `src/prototype.rs`: `HeroMotion`, `Hero::motion`, and `step_hero_driven` reading it;
- `src/agility.rs` (new): the loader, the validation, the FNV pin and `NO_KNOB`, mirroring `src/cf02.rs`;
- `assets/data/gameplay/hero_agility_v0.json` (new): DM-Q01, Q02, Q08, plus the evasion keys Q11..Q16 already present so DM-B does not reshape the file;
- `src/party.rs`: `MotionProfile`, the `donut_motion` field and `with_cf02_agility()`;
- `src/corridor.rs`:
  - setting `donut_body.motion` on build, restart and respawn;
  - the separation wall using per-body speed (DM-Q09);
- `src/follower.rs`: the per-hero jump reach (DM-Q08);
- `src/companion/nav.rs`: `NavConfig::agile()` and the agile replay test;
- `src/companion/navfollow.rs`: preset selection only;
- `src/bin/floor01_scene.rs`: `live_party` applies `with_cf02_agility()` under `--cf02`;
- `src/playtrace/replay.rs` and the reset descriptor: the `donut_motion` field, absent meaning legacy;
- `src/autoplay/party/scenario.rs`: the conditional config hash;
- `src/cf02_fixtures.rs` and `src/cf02_fixtures/engineer_tests.rs`: `ARENA_SOLO` and `ARENA_COOP` gain `.with_cf02_agility()`, and the 8 listed goldens are rebaselined;
- new `src/agility_fixtures.rs` with the `dm_` motion fixtures and their goldens, plus its `pub mod` line in `src/lib.rs`;
- `docs/01-design/character-progression-horizon.md`: one change-record row and the Floor 01 line, per spec section 8.

**Must not touch:** `src/combat_scene.rs` (DM-B's file), `src/companion/director.rs` and the companion profiles (CA2's files), `cf02_combat_v0.json` (its `TUNING_HASH` must not move).

**Acceptance:** spec section 7, items 1 to 5, 11 (motion part) and 12 (motion part). The PR body carries:

- the nav diff: Carl's edges identical, plus the list of new and lost Donut edges;
- the old and new hash of each of the 8 moved goldens;
- a statement that no other golden moved;
- a Donut row for the Engineer counterplay (spec section 2.1);
- the horizon-fit line.

## Stage DM-B — Evasion after the biscuit (code PR 2)

**Goal:** after the biscuit unlock, under Agile, the selected `dm_q11_evasion_mode` evades evadable attack instances per spec section 3, whether a human or the AI plays her. Before the biscuit nothing changes.

**Files owned:**

- `src/combat_scene.rs`: the evasion check in `swing_on`, the Engineer ram path and `step_globs`, in the order of DM-Q20, with instance keys;
- `src/combat.rs`: the `CombatEvent::HeroEvaded` and `EvadeArmed` variants;
- `src/agility.rs`: the reflex state, the counter-keyed roll and the hash fold gated on Agile;
- `src/bin/floor01_scene.rs`: the `--donut-evasion` flag, accepted only with `--cf02`;
- `src/playtrace/replay.rs` and the reset descriptor: the `evasion_mode` field;
- `src/agility_fixtures.rs`: the evasion and biscuit-gate `dm_` fixtures (spec section 5) and the share-measurement test (spec section 3.3, non-golden).

**Must not touch:** the hurt paths of non-evadable sources, beyond resetting `last_reset_tick` on any `HeroHurt`, and `src/donut_mana.rs` (read `unlocked()` only).

**Acceptance:** spec section 7, items 6 to 12.

- The PR body carries the share table, the adopted `rearm` (480 unless the sweep says otherwise) and the measured share at roll 80 and at 150.
- **No existing golden moves.** The 8 arena goldens of DM-A start locked.

## Stage DM-C — Presentation stand-ins and evidence (code PR 3)

**Goal:** the cues of spec section 3.4 and the GIFs of spec section 7.

**Files owned:**

- the Bevy presentation adapters for Donut's sprite: the tail glint, the evade arc with afterimages and the smug trigger. Read-only from sim state and events.
- `docs/evidence/donut-mobility-01/` with the GIFs, a contact sheet and `SHA256SUMS`.

Checks: the run cycle is slip-free at 207 px/s, and the jump phases sequence at a 485 launch.

**Acceptance:**

- GIFs 1 to 4 and the contact sheet, all at gameplay scale.
- A replay with presentation on and off gives an identical state hash.
- The `Rights/Provenance` section of the PR body names the stand-ins as code-drawn overlays on existing approved frames, with no new art.

## Order and grouping

1. **DM-A first.**
   - It may run in parallel with CA2-A only if CA2-A does not edit `nav.rs`'s `NavConfig` or `navfollow.rs`'s graph build. Otherwise CA2-A rebases on DM-A.
   - CA2's revision of PR #172 absorbs spec section 4 before CA2-A is briefed.
2. **DM-B** after DM-A. It is independent of CA2: it does not touch companion files and can run alongside CA2-A or CA2-B.
3. **DM-C** after DM-B.
4. **Grouping:** speed mode allows grouping DM-A and DM-B into one PR if the reviewer agrees. Keep DM-C separate, because evidence regenerates on the final head.
5. **Owner handoff:** after DM-C, the orchestrator sends the owner:
   - the GIFs;
   - the measured evasion share;
   - the A/B command line, `--cf02 --donut-biscuit --donut-evasion reflex|roll`;
   - the two owner-level questions of spec section 8.
6. **DM-ART:** only after the owner chooses a mode.

## Definition of done

- Every spec section 7 item has a test, or an evidence file named in a PR body.
- CI is green on each final head.
- The golden changes are exactly those spec section 5 lists.
- The backlog records "merged, game-feel review pending" (the orchestrator does this after merge).
