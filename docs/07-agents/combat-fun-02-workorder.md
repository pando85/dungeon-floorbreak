# Agent work order: COMBAT-FUN-02 (post DEMO-01)

**Status:** design adopted (CF02-Q01..Q33, orchestrator under the owner's design-defaults directive, 2026-11-20); implementation not started. The design PR changed no runtime source, art source, atlas, controller binding, tuning table or authoritative simulation. The [design](../01-design/combat-fun-02-post-demo01-proposals.md) holds every adopted value; this work order splits delivery into small PRs with disjoint file ownership.

**Goal:** deliver one compact, replayable combat-focused microarena (`floor01_scene --cf02`) in which the owner enjoys basic attacks, enemy collisions, ground advantage, Donut participation and authored environmental kills significantly more than in DEMO-01, with clear original-pixel deaths and credible evasive choices. Do not ship a second long horizontal systems corridor.

## Read first, in this order

1. `AGENTS.md`, `docs/07-agents/workorders.md`, `docs/07-agents/agent-orchestration.md`, then the [owner playtest](../04-production/demo-01-playtest-2026-11-20.md) and the [design](../01-design/combat-fun-02-post-demo01-proposals.md) section 7 (the adopted values for your PR only).
2. `docs/01-design/combat-first-encounter-design.md`, `combat-feel-hybrid-physics.md`, `mvp-controller-contract.md`, `character-progression-horizon.md` and `antihero-tone.md`; do not supersede approved controls or chronology.
3. `docs/04-production/demo-01-plan.md` (B1–B6), `docs/03-tech/floor01-scene.md` and `docs/01-design/party-2p-open-questions.md` (Q7–Q9), relevant sections only.
4. Per PR: `docs/02-art/donut-moveset-q10.md` (adopted defaults Q10-A01..A10), `docs/01-design/donut-mana-v0-proposal.md`, `docs/03-tech/death-d-finishes-spec.md`, `docs/03-tech/cinematic-finishers-cine-01.md`, `tools/art-references/death-lab/README.md` (open its `index.html` at intensity 4), `docs/03-tech/monster-death-system-proposal.md`, `docs/07-agents/pixel-art-pipeline.md`.
5. The current `main` tree. Grep before reading; `src/combat_scene.rs` and `src/bin/floor01_scene.rs` are large.

## Ground rules for every PR

- **Gate:** every CF02 rule reads `cf02_rules` (default `false`) or its own knob, except CF02-Q18 (llama targets, a #123 follow-up that applies everywhere). With the switch off, every existing golden is byte-identical: the 20 combat `EXPECTED_HASHES`, `GOLDEN_WALK_RIGHT_400`, the story hash, the party, boss, `q10_*`, `demo01_*` and CINE goldens, and the whole-route autopilot hash `0363078d58558e95`.
- **Data:** knob names equal the question IDs (for example `q02_stomp_startup`). They live in `assets/data/gameplay/cf02_combat_v0.json` (loaded, validated, FNV-1a pinned like `companion_magic_v0.json`) or in `PartyTuning` for the party knobs. A load error names the field.
- **Events:** new `CombatEvent` and `MoveId` variants are **appended**, mixed into the hash only when emitted. Fixed order on one tick: Carl, Donut, goblins, llamas (as today).
- **Fixtures:** new ones use the `cf02_*` namespace, committed final hashes, replayed twice, headless and in the render loop.
- **CI gate (every PR, on the final head):** `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`, `bash scripts/ci_floor01_combat.sh headless`, and the GitHub workflows `documentation`, `game`, `floor01-scene` (plus `art-assets` for art PRs) green.
- **Review and merge:** an independent top-tier review per PR, then a fix check, then squash-merge by the orchestrator under the AGENTS.md delegation. Gameplay is recorded as "merged, game-feel review pending".
- **Model tiers:** gameplay code and tests: medium tier (Sonnet); top tier only if a medium-tier attempt fails. Pixel art: top-tier design keys and spec, medium-tier execution, separate top-tier review. Evidence, captures, checksums, README rows: medium tier.

## PR sequence

| PR | Scope (question IDs) | Owns (files) | Depends on | Fixtures and goldens | Evidence | Tier |
| --- | --- | --- | --- | --- | --- | --- |
| **CF02-00** baseline and scaffold | Gap report (Q23 audit: B2 contact, B3 decap, B5 blast, B4 llama heavy/routine, camera pan/kick/zoom traces); scorecard template; empty `cf02_combat_v0.json` loader with `cf02_rules: false` and the `--cf02` flag loading the DEMO hall for now | `src/cf02.rs` (new: knobs, loader), `assets/data/gameplay/cf02_combat_v0.json`, `docs/evidence/cf02/`, the arg parsing block of `src/bin/floor01_scene.rs` | main | Parity test: `--cf02` with every rule off replays all goldens | Gap report with event log vs camera visibility per beat; 640x360 captures of each missed spectacle; camera trace table | Sonnet |
| **CF02-C1** llama targets (WORK-023-F4) | Q18 | `src/combat_scene/donut.rs`, `src/llama.rs`, `src/donut_moves.rs` (target list only), the llama notes in `docs/04-production/demo-01-plan.md` and `demo-01-questionnaire.md` (Q12d) | main | `cf02_llama_swat_interrupt`, `cf02_llama_swat_guard`, `cf02_llama_pounce`, `cf02_llama_bolt_stops`; parity test proves no existing golden moves | 2P capture: Donut interrupts a llama tell, bolt ends on the llama | Sonnet |
| **ART-CARL-STOMP** | Carl stomp clip for Q02 (knee-up 9, stamp 3, recover 14), plus the follow-through kick (Q05: decide whether the 9-tick startup skips the kick's chamber frame or needs one new frame) | `assets/source/characters/carl/` (new stomp grid and `.ase`), its manifest rows, `docs/02-art/carl-stomp-spec.md` (new) | none (spec numbers are in Q02/Q05) | Art validators (bounds, pivots, palette, foot contact) | Contact sheet 1x/3x, timing diagram, in-scene GIF at 1x | Opus design, Sonnet execution, Opus review |
| **CF02-A** Carl branches and ground state | Q01–Q08 | `src/combat.rs` (`MoveId::Stomp`, `CARL_STOMP`, follow-through kick spec), `src/feel.rs`, `src/goblin.rs` (lying hurtbox, CF02 down time, lethal knock by move), the Carl press handling in `src/combat_scene.rs`, the Down + Physical sample in the `RawInput` adapter of `src/bin/floor01_scene.rs` | CF02-00 | `cf02_kick_then_stomp`, `cf02_stomp_whiff`, `cf02_stomp_standing`, `cf02_stomp_no_stand_up`, `cf02_confirm_kick`, `cf02_whiff_no_confirm`, `cf02_jab_over_lying`, `cf02_lethal_kick_far`, `cf02_stomp_boss` (boss super armour), a 2P same-tick case | Debug-box captures vs the Q02/Q03 boxes; 1x GIF of the three branches; placeholder kick frames for the stomp until ART-CARL-STOMP lands (labelled) | Sonnet |
| **CF02-C2** assistant and swap | Q13–Q17, Q19, Q20 | `src/follower.rs`, `src/party.rs`, `src/scroll_camera.rs` (eased pan), the follower read-only threat view in `src/corridor.rs`, AI press injection in `src/combat_scene/donut.rs` (after CF02-C1 merged) | CF02-C1 (llama assist), CF02-00 | `cf02_assist_swat`, `cf02_assist_no_kill`, `cf02_assist_offscreen_idle`, `cf02_assist_cooldown`, `cf02_assist_off_in_2p`, `cf02_swap_eased_pan`; parity: assist off replays the party goldens | Solo GIF with the AI Donut interrupting; kill-credit table (player vs AI) per fixture; pan trace old vs new | Sonnet |
| **CF02-B** contacts and walls | Q09–Q12 | `src/goblin.rs` (knockdown target, wall slam), `resolve_contacts` and the flier loop in `src/combat_scene.rs`, `CombatEvent::BodyContact` / `WallSlam` in `src/combat.rs` | CF02-A (same files; serialize) | `cf02_contact_knockdown`, `cf02_contact_below_threshold`, `cf02_no_chain`, `cf02_body_bowl`, `cf02_wall_slam_kill`, `cf02_wall_slam_body`, `cf02_layout_edge_no_slam`; CINE test: `BodyContact` never scores `CINE-CONTACT-SETUP` | 640x360 first-person capture of a kick, the second body falling and a wall slam, readable without HUD text | Sonnet |
| **ART-GOB-SLAMMED** | Q21 poses (stomped, wall crumple, flung) and Q22 preset-4 budget | `assets/source/characters/goblin/` death sources, `assets/data/death/species_death_profiles.json` (one row), `src/death_present.rs`, `src/gore.rs` (heavy budget) | CF02-B event shape (names fixed by this design, so the design step can start with CF02-A) | Death profile tests; event/hash parity across presets 1–4 and CINE on/off | Contact sheets 1x/3x per pose and preset; blind check: three finishes named correctly | Opus design, Sonnet execution, Opus review |
| **CF02-AUDIO** | Q24, Q27 cue | `assets/audio/floor01/` (new cues, cue map, manifest, QA report), `src/audio.rs`, `src/audio_adapter.rs` | CF02-A, CF02-B (events exist) | Cue-mapping unit tests; structural QA (levels, loops, overlap limit) | Short capture with sound per cue; QA report; owner listening stays open (design §8) | Sonnet |
| **CF02-F** microarena | Q26–Q33 | `assets/levels/floor01/cf02_microarena.room.json`, `src/cf02.rs` (spawns, waves, exit), `src/shore.rs` (hero damage, goblin self-own), scenario wiring in `src/bin/floor01_scene.rs` | CF02-A, CF02-B, CF02-C2 | `cf02_arena_direct`, `cf02_arena_environment`, `cf02_arena_coop` (2P), `cf02_block_hurts_carl`, `cf02_block_spares_striker`, `cf02_goblin_self_own`, `cf02_intact_shore_ignores_goblins`, `cf02_parapet_cover`, `cf02_perch_above_spit`, whole-room autopilot hash; reachability in every shore state | 640x360, 640x400 and 1920x1080 captures; widest auto-camera with no void; 1P swap and 1P assistant GIFs | Sonnet |
| **CF02-REVIEW** | Integration, scorecard, owner handoff | `docs/evidence/cf02/`, `docs/04-production/backlog.md`, playtest scorecard | all | Re-run of every golden and `cf02_*` fixture on the final main | Frame-time note on real hardware; real two-pad recording or **NOT RUN**; owner instructions to play the room three times, once per approach | Sonnet execution; Opus final review |

### Parallel groups

Run at most three at once; a later group starts when its dependencies have merged.

| Group | PRs in parallel | Why they do not collide |
| --- | --- | --- |
| 1 | CF02-00, CF02-C1, ART-CARL-STOMP (design step) | `src/cf02.rs` + arg parsing; Donut adapter + llama; art sources only |
| 2 | CF02-A, CF02-C2, ART-CARL-STOMP (execution and review) | Carl/goblin/combat files; follower/party/camera files and the Donut adapter; art sources |
| 3 | CF02-B, ART-GOB-SLAMMED (design step) | Combat files (after A); death art spec only |
| 4 | ART-GOB-SLAMMED (execution and review), CF02-AUDIO | Death sources/presenter/gore; audio pack and adapter |
| 5 | CF02-F | Room, shore, scenario wiring |
| 6 | CF02-REVIEW | Docs and evidence |

**Serialization rule:** `src/combat.rs`, `src/goblin.rs`, `src/combat_scene.rs` and `src/bin/floor01_scene.rs` have one owner at a time: CF02-00 (arg block only), then CF02-A, then CF02-B, then CF02-F. A PR that finds it must touch a file owned by a running PR stops and reports instead of editing it.

## Per-PR acceptance (from the design)

- **CF02-00:** one factual gap report from logs and captures; no speculation promoted to an observed finding.
- **CF02-A:** three distinct branches (on-the-spot, follow-through, ground finish) at 1x without a new mandatory button or stun-lock; ordinary goblin time-to-kill unchanged for jab -> haymaker.
- **CF02-B:** a 640x360 capture shows kick, second body down and wall slam without console or HUD copy; no duplicate effects per (launch, target); no chains.
- **CF02-C1/C2:** the AI never lands a killing blow, never acts off-screen, never casts before the biscuit; kill-credit and interrupt counts per fixture; no change in 2P.
- **ART PRs:** the art acceptance targets in each spec; Carl stays an effortful everyman (antihero review gate), no new Donut pixels.
- **CF02-AUDIO:** each Q24 cause has its own cue; overlap limit holds; no cue drives gameplay.
- **CF02-F:** a player can name the upcoming shore hazard without coaching; at least two llama answers usable; solo and 2P completable in every shore state; no void at the widest view or 640x400.
- **CF02-REVIEW:** equal outcomes under gore 1/4, CINE on/off and every camera; no unbounded contacts; owner handoff lists what was NOT RUN.

## Required report (each agent)

1. Feature and **visible** result, with the A/B baseline (CF02 off vs on).
2. Branch, head SHA, and every authoritative event, move or input added.
3. Commands actually run and CI status; never claim unrun controller tests.
4. Before/after hashes; confirmation that every legacy golden is unchanged.
5. Original pixel/audio evidence, if affected.
6. Remaining risks and the next dependent PR.
7. Specific answer: **why is this more fun, at 1x, without adding a new mandatory button?**

**Rights:** use only already-authorized original project sources; no copied franchise or other game's art/music/sounds. Commercial distribution rights remain unresolved. **This document requests no new images or ZIP intake.**
