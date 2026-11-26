# FLOOR01-TROG-RUNUP-01: local agent work order (TROG-A, TROG-B, AREA-TROG-01)

**Task:** make the troglodyte territory and the Test of Strength playable in the floor run, between the survivors' camp and the Juicer. Three new enemies run on ENEMY-SLOT. The numbers, contracts, pins, fixtures and gate are in [the troglodyte integration plan](../01-design/floor-01-troglodyte-integration.md); this order only sequences them. Its section numbers are cited as "plan 5.4" and so on. Every result is a labelled greybox, "merged, game-feel review pending".

**Provenance.** This order comes from the owner's draft PR #191 ("The House of Broken Weights", `docs/07-agents/floor-01-troglodyte-gym-workorder.md` on branch `design/f01-troglodyte-runup-juicer-gym`). That PR stays open and unmerged as the record of the original text. This copy keeps:

- the draft's priorities: one fight per species first, then the mixed fight, then the hand-off;
- its failure conditions;
- its autonomous-testing duty;
- its "no boss tuning" rule.

It changes four things:

- TR-A0..TR-F map onto two Sonnet PRs and one art order;
- the lead-in is two run rooms, and #191's T5 is the Juicer arena's existing zone A (plan section 15);
- the bench uses the merged CORRAL-D partition, with a declared static fallback (plan 5.4);
- the stationary probe becomes an asserted gate on the Test of Strength (plan 7.3).

## Read and branch coordination

Follow `AGENTS.md`, [the orchestration guide](agent-orchestration.md) (speed mode, local-validation merge) and the merged code **before touching any shared module**.

**Audit:**

- ENEMY-SLOT: `src/scene_enemy.rs`, `src/combat_scene/slot.rs`; the Hoarder adapter `src/combat_scene/pile_boss.rs` for the `env_stimuli` shove path;
- the roster parse in `src/encounter.rs`;
- the partition in `src/env_prop.rs` and `src/combat_scene/env.rs` (`reset_keeping_partitions`);
- the companion view kinds and the llama opt-in feed in `src/combat_scene/companion.rs`;
- `src/corral_eval/` and `src/bin/corral_eval.rs`;
- the run: `src/f01_late.rs`, `src/f01_run.rs`, `assets/levels/f01_late/manifest.json`, `src/f01_rest.rs` (room module pattern);
- `src/bin/floor01_scene.rs` and the PLAYTRACE dispatch;
- `assets/manifests/trog-0{1,2,3}.json` and the TROG-01/02/03 specs.

**Coordination on current main:**

- **The Juicer is frozen.** Room `juicer`, scene `boss_f01`, `assets/data/bosses/brute_boss_f01.json`, its gate, its goldens and its data pins must not move. A move is a stop condition.
- **No forks of shared code.** The trog family is one slot kind with three profiles. Do not clone the boss controller, the llama machine or the goblin AI. Do not add trog logic to companion files. The feed consists of:
  - the two default trait methods;
  - an appended `EntityKind::Slot`;
  - its one data-free `mode_for` arm in `src/companion/autonomy.rs`, a copy of the Goblin arm (plan 5.8).
- **No silent rewrites** of Playtrace, the render replay, CF02, the camera or `corral_eval`'s corral output (byte-identical, plan 7.2).
- **`SlotStepCtx` constructors.** The appended `last_tell_start` field must be set in every struct literal. At the plan's base these are `step_slot_enemies` in `src/combat_scene/slot.rs` and the test helper `step_ctx` in `src/combat_scene/pile_boss.rs`. Re-grep `SlotStepCtx {` at the branch point and list every hit in the PR (plan 12.1).
- **Each PR re-pins `MANIFEST_HASH` once** from main's value at its branch point (`0xd213_db6d_b329_d342` at the time of this plan). Rebase and re-count if another manifest PR lands first.

## Deliverables, mapped onto the PRs

| #191 item | Deliverable | Evidence / pass condition | PR |
| --- | --- | --- | --- |
| TR-A0 audit | The audit above, recorded as a short "what main holds" note in the PR body | Real file paths and the commands run | TROG-A |
| TR-B1 Pygmy | Coil, committed leap on the airborne-pivot parabola, and landing recovery (plan 5.2), leash, interrupt. The landing is punished low: the CF02 stomp or the pounce, then a jab in Rest (TROG-AD22) | TROG-T03, F01, F02 (positive control) | TROG-A |
| TR-B2 Basher | Heavy tell, smash, 20-tick recovery, the interrupt window and budget, no magic immunity | TROG-F03, F04 | TROG-A |
| TR-B3 Virtuoso | Feint, lash and retract, with the band clipped at solids and the dead zone inside +56. It targets only a hero its band can reach, so it never tells at a grounded Donut (TROG-AD17, TROG-Q13) | TROG-F05 (the +0/+32/+64 matrix), F06 | TROG-A |
| TR-C level | `trog_run` (#191 T0..T3) listed after `rest_camp`; the run grows to 16 rooms | TROG-F07, F08, T04, T05; Bevy captures | TROG-A |
| TR-D mixed | `trog_strength`, the bench partition, the Basher's smash shove and rule (b), or the declared static fallback | TROG-F09..F15 | TROG-B |
| TR-E gym threshold | The hand-off: the room-clear panel, then relaunch into the unchanged `boss_f01`. The reception and turnstile are the arena's zone A (no new room) | TROG-T06 run walk; boss fixtures unchanged (F17) | TROG-B |
| TR-F evaluation | `trog_eval`: the stationary policies, mobile Donut, Carl melee, the co-op probe; the gate of plan 7.3 | The gate asserted; JSON and text table; corral report byte-identical | TROG-B |
| Art | The environment kit for both rooms: AREA-TROG-01 (plan section 16) | The art pipeline's own gates | AREA-TROG-01 |

## The three orders

### TROG-A (Sonnet): the family and `trog_run`

Implement plan 12.1. Land the enemies with their isolated fixtures first, then the room, then the run listing.

**Cut order** (cut from the top; never cut the commit rule, the leash, the wake-in-view rule or the run listing):

1. the atlas wiring (keep the labelled hurtbox rectangles);
2. the companion feed (the bot then ignores troglodytes, which is reported);
3. the T3 platform (keep the step; F05 still runs in its own fixture geometry).

### TROG-B (Sonnet): the Test of Strength, the gate and the hand-off

Implement plan 12.2.

**First action:** build the 80 px `bench_stack` partition in a unit test and run it through the sidecar validator. If either rejects it, switch to the static fallback (plan 5.4), record it as an implementation note, and continue. Do not widen the partition rules.

The cut order is plan 12.2's.

### AREA-TROG-01 (art pipeline: Opus design, Sonnet execution, Opus review)

Queue it in the area kits plan. Plan section 16 lists:

- the needs;
- the kits to reuse (ENV-01 catacomb, AREA-GYM-01 gym);
- the new pieces: knuckle-mark and impact decals, the fist-shaped hole, the bell, a stone-to-tile threshold, `gym_bench_stack` intact and remnant, a small debris burst.

The greybox rooms do not wait for it. Its sprites must not change any sim value: the bench's collision is the sidecar's.

## Technical guardrails

- **Wake and target truth.** A proximity wake and every tell need the enemy fully in view. A hit on a Dormant trog wakes it into Rest, and the cause is logged (TROG-AD21). A tell names only a hero whose hurtbox can meet the attack band (TROG-AD17).
- **Commit at tell start.** Facing, landing x, smash rect and line are fixed on the tell's first tick. One blow per hero per attack instance, no tracking. A blow before its active window is a hard failure (plan 7.3 row 5).
- **Interrupts key on `HitSource`, never on poise.** Bolts never interrupt the Basher or the Virtuoso. The Basher's window and budget stop a stunlock (plan 5.2).
- **The bench breaks only by collision:** a hero's melee tokens or the Basher's smash rect overlapping it by at least 12 px.
  - Never by proximity, a timer or a script, and never more than once.
  - Always attributed: the shove's `credit` is the baiting hero's player, or `None` for the bot (TROG-AD26).
  - Rule (b) obeys every tell gate.
- **No rescue, no teleport, no invincibility.** A bot stuck behind the bench is reported, not moved.
- **Tick order is unchanged:** scene, then slot, then env. A broken bench stops colliding on the next tick (plan 5.7).
- **The restore rules of plan section 10:** a respawn sends living troglodytes home and keeps the bench's state; a restart rebuilds all.
- **Data and determinism.** Every time, speed and distance is data in `enemy_trog_v0.json` with a pin. Boxes and windows are checked against the art manifests. Presentation never changes the log or the hash.
- **No invented content:** no poison, no gore escalation, no slow motion, no boss bar, no dialogue, no audio (none exists), no Juicer tuning.

## Exact acceptance questions

1. Does each species **commit in one direction** at its tell start, so a hero who moves during the tell is not hit? (TROG-02, 06)
2. Can solo Carl and solo Donut each beat each species with ordinary moves in a short fight, with no immunity and no stunlock? (TROG-03, 05, 07)
3. Does the Virtuoso's tongue truly pass over grounded Donut, hit her mid-hop, and stop at solids? Does it pick Carl over a nearer grounded Donut? (TROG-06)
4. In the Test of Strength, are two tells ever live on one hero at once? Is there always a safe response? (TROG-08)
5. Does the bench break only from a real overlap or a hero's hands, at most once, with the right credit? Can the room be cleared with the bench untouched? (TROG-09, 10)
6. Does the run walk `rest_camp` → `trog_run` → `trog_strength` → `juicer` with the ledger unchanged, and does the unchanged boss scene take over its own gate? (TROG-11)
7. Does the gate of plan 7.3 pass (no stationary Donut clear of both troglodytes from a natural start), while mobile Donut, Carl melee and co-op succeed? (TROG-14, 15)
8. Are fixed-tick replays exact after a respawn and a restart? (TROG-13, 17)
9. Does the mixed fight feel like a test of reading bodies rather than a grind? Ask for a recorded human verdict; this one is never answered by automation.

**Do not claim owner approval** from an automated metric. Each PR keeps:

- the TROG matrix with PASS, FAIL, BLOCKED or NOT RUN per row;
- the actual commands;
- native captures;
- the stop-condition checks;
- a short honest "not yet working" section.

**Minimum good outcome.**

- **TROG-A:** three enemies that fight honestly in isolation and a listed `trog_run`.
- **TROG-B:** a listed `trog_strength` that can be cleared directly, a passing stationary gate, and an unchanged Juicer behind it.

The bench smash may land as the declared static fallback if the partition cannot carry it; that keeps the beat open, not done.
