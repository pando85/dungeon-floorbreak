# DONUT-MOBILITY-01 — Implementation work order

**Status:** specification only; nothing is implemented. **Revision 1** (2026-11-21): review fixes and owner decisions, see spec section 9. **Priority:** P1, owner request 2026-11-21 ("implement it": DM-A, DM-B, DM-C run after the design PR merges). It lands **before CA2-A** of COMPANION-AI-02 (section "Order").
**Authority:** [design spec](../03-tech/donut-mobility-01.md). The DM-Q values there are adopted defaults: do not re-decide them. A design problem found during execution goes back to the top tier.
**Read first:** [AGENTS.md](../../AGENTS.md), [orchestration](agent-orchestration.md) (standard brief, speed mode), the spec sections each stage names.

## Routing

| Step | Tier | Notes |
| --- | --- | --- |
| Design (done: the spec, Revision 1) | Top tier | No further top-tier design until the owner's game-feel review, except design defects found in review |
| DM-A, DM-B, DM-C execution | Medium tier | Code, tests, fixtures, goldens, evidence, docs |
| Review of each PR | Independent top-tier agent | Against the spec and its section 7. Speed mode: a medium-tier fix check suffices unless the review found a design or logic defect |
| DM-ART (after DM-C) | Per the [pixel-art pipeline](pixel-art-pipeline.md#routing-by-difficulty) | The owner chose the reflex, so the keys can be designed |
| DM-RACK (follow-up) | Medium tier, top-tier review | Enables the boss right rack (spec section 5); not part of this order |

Every stage:

- has **no motion gate**: the agile motion is Donut's default everywhere (owner decision 2026-11-21). No CLI flag, data switch or `--cf02`-only path selects legacy motion. `MotionProfile::Legacy` is constructed only by tests and by the playtrace rebuild of pre-DM recordings;
- keeps evasion gated on the biscuit (`DonutMana::unlocked()`);
- changes a golden or a pinned value only with an entry in the PR's before/after table, and never loosens an assertion;
- runs `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets` and `scripts/ci_floor01_combat.sh headless`;
- leaves `docs/04-production/backlog.md` alone: speed mode, the orchestrator updates it after merge, and the PR body carries the status.

## Stage DM-A — Agile motion everywhere (code PR 1)

**Goal:** in every scene where Donut has a body, she runs 207 px/s and jumps 500 px/s, human or AI, with no off switch in normal play. Carl is bit-identical. Every golden where Donut's body moves is re-pinned on purpose, and the trail runs are proven unchanged.

**Files owned:**

- `src/prototype.rs`: `HeroMotion` (a value, not a `Hero` field), `step_hero_motion`, and `step_hero_driven` as the `HeroMotion::CARL` wrapper;
- `src/agility.rs` (new): the loader, the validation (rearm range 600..=900, run below 240), the discrete-apex helper, the FNV pin and `NO_KNOB`, mirroring `src/cf02.rs`;
- `assets/data/gameplay/hero_agility_v0.json` (new): DM-Q01, Q02, Q08, plus DM-Q11 and Q12 already present so DM-B does not reshape the file;
- `src/party.rs`: `MotionProfile`, the `donut_motion` field, `Agile` in `PROVISIONAL_TUNING`, and a test-only legacy tuning;
- `src/corridor.rs`:
  - `step_bodies_driven` steps Donut's body with `self.drive.tuning.donut_motion` every tick (no stored motion, no reset sites);
  - the separation wall using per-body speed (DM-Q09);
- `src/follower.rs`: the per-hero jump reach (DM-Q08). Carl keeps `JUMP_REACH_PX` exactly; Donut uses her discrete apex minus `dm_q08_ai_jump_margin_px`;
- `src/companion/nav.rs`: `NavConfig::agile()` and the agile replay test;
- `src/companion/navfollow.rs`: build with `NavConfig::agile()`, and `approach()` / `steer()` take the body's run speed (spec section 4, F4);
- `assets/levels/floor01/cf02_microarena.room.json` and its loader in `camera_lab::layout`: the optional `ceiling_px` field (absent = none), set to 228 for `CF02-ARENA` (DM-Q27);
- `src/playtrace/live.rs`, `manifest.rs`, `replay.rs` and the reset descriptor: `content_hashes["agility_tuning"]`, `feature_flags["donut_motion"]`, the descriptor field `donut_motion` (absent = legacy, `skip_serializing_if`), and the rebuild;
- `src/autoplay/party/scenario.rs`: fold `donut_motion` into the config hash only when it is not `Agile`;
- every golden table and pinned test of spec section 5, re-pinned, and new `src/agility_fixtures.rs` with the `dm_` motion fixtures and their goldens, plus its `pub mod` line in `src/lib.rs`;
- the committed replays and evidence of spec section 5 (`docs/evidence/cf02/replays/`, `companion-ai/`, `party-2p/` with `SHA256SUMS` and README hash lines, `boss-f01/`, `donut-q10-mana/` with `SHA256SUMS`, `audience-01/` if it moves, the hash lines in `docs/evidence/cf02/README.md`), rewritten explicitly;
- the evidence scripts with tick windows (`scripts/make_cf02_i_evidence.sh`, `cf02_arena_captures.sh`, `cf02_render_match.sh`, `ci_floor01_render_smoke.sh`, `combat_lab_evidence.sh`): windows re-measured, the committed state files regenerated;
- `docs/01-design/character-progression-horizon.md`: one change-record row and the Floor 01 line, per spec section 8;
- `docs/01-design/party-2p-open-questions.md` and `docs/02-art/donut-moveset-q10.md`: Q10's Move and Jump rows marked closed by DONUT-MOBILITY-01 (Jump adopted at 500, Move superseded by 207). The boss doc keeps its text; the rack note points at DM-RACK.

**Must not touch:** `src/combat_scene.rs` evasion paths (DM-B), `src/companion/director.rs` and the companion profiles (CA2's files), `cf02_combat_v0.json` (its `TUNING_HASH` must not move), `brute_boss_f01.json` (the rack stays off).

**Acceptance:** spec section 7, items 1 to 8, 13 (motion part) and 14 (motion part). The PR body carries:

- the **full before/after table** of every pinned value in the repository (spec section 5): old, new, and "moved: Donut walks or jumps", "moved: other cause (named)" or "unchanged" with its reason. The trail and Carl-only pins appear as "unchanged";
- the nav diff: Carl's edges identical, plus the new and lost Donut edges per space;
- the HUD table of spec section 2.1 as measured;
- the U4 measurement: Carl's off-screen ticks and regroup count on the corridor kit with a running human Donut;
- a Donut row for the Engineer counterplay (spec section 2.1);
- the horizon-fit line.

## Stage DM-B — Reflex evasion after the biscuit (code PR 2)

**Goal:** after the biscuit unlock, the reflex evades evadable attack instances per spec section 3, whether a human or the AI plays her. Before the biscuit nothing changes.

**Files owned:**

- `src/combat_scene.rs`: the evasion check in `swing_on`, inside `ram_on_hero`'s candidate loop, and in `step_globs`, in the order of DM-Q20, with instance keys;
- `src/llama.rs`: `Glob::spawn_tick`, set at spawn, not hashed (F8);
- `src/combat.rs`: the `CombatEvent::HeroEvaded` and `EvadeArmed` variants;
- `src/agility.rs`: the reflex state, the reset rule of DM-Q13 and the hash fold from the first `EvadeArmed`;
- `src/agility_fixtures.rs`: the evasion and biscuit-gate `dm_` fixtures (spec section 5) and the share-measurement test (spec section 3.3, non-golden).

**Must not touch:** the hurt paths of non-evadable sources, beyond resetting `last_reset_tick` on any `HeroHurt`, and `src/donut_mana.rs` (read `unlocked()` only). No CLI flag.

**Acceptance:** spec section 7, items 9 to 14.

- The PR body carries the share table at rearm 600, 720 and 900, with the per-wave and per-attempt counts and the first-hit-of-wave count. 720 stays the default.
- It lists every golden that moved because Donut armed at least once, with old and new values, and states that no other golden moved.

## Stage DM-C — Presentation stand-ins and evidence (code PR 3)

**Goal:** the cues of spec section 3.4 and the GIFs of spec section 7.

**Files owned:**

- the Bevy presentation adapters for Donut's sprite: the tail glint, the evade arc with afterimages and the smug trigger. Read-only from sim state and events.
- `docs/evidence/donut-mobility-01/` with the GIFs, a contact sheet and `SHA256SUMS`.

Checks: the run cycle is slip-free at 207 px/s, and the jump phases sequence at a 500 launch and through a DM-Q27 ceiling bonk.

**Acceptance:**

- GIFs 1 to 4 and the contact sheet, all at gameplay scale.
- A replay with presentation on and off gives an identical state hash.
- The `Rights/Provenance` section of the PR body names the stand-ins as code-drawn overlays on existing approved frames, with no new art.

## Order and grouping

1. **DM-A first.** CA2-A and CA2-B rebase on DM-A: DM-A owns `NavConfig::agile()`, the navfollow preset and the run-speed parameter of `approach()` / `steer()`, which CA2's go-to entry point reuses. CA2 records its goldens after DM-A merges (CA2 Revision 1 already says so).
2. **DM-B** after DM-A. It touches no companion file and can run alongside CA2-A or CA2-B.
3. **DM-C** after DM-B.
4. **Grouping:** speed mode allows grouping DM-A and DM-B into one PR if the reviewer agrees. Keep DM-C separate, because evidence regenerates on the final head.
5. **Owner handoff:** after DM-C, the orchestrator sends the owner:
   - the GIFs;
   - the measured share table (no target), with one sentence on the opener-of-a-wave effect (spec section 3.3);
   - the playtest line `--cf02 --donut-biscuit`, and the note that under `--cf02` only that flag unlocks the reflex; the earned-at-the-guild flow is exercised by the `dm_evasion_unlocks_at_guild` fixture, because the story run has the guild but no enemies;
   - the two owner-level questions of spec section 8.
6. **DM-ART** and **DM-RACK** after DM-C.

## Definition of done

- Every spec section 7 item has a test, or an evidence file named in a PR body.
- CI is green on each final head.
- Every changed golden or pinned value appears in its PR's before/after table, with no loosened assertion.
- The backlog records "merged, game-feel review pending" (the orchestrator does this after merge).
