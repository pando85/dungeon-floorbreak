# COMBAT-LAB-01 — Manual playtest protocol, evidence, and ML capture quality

**Status:** planned, 2026-11-20. No completed COMBAT-LAB-01 human session is claimed here.
**Playtest focus:** mid-progression Carl + Donut, vertical combat, real enemy variety, rare spectacular finishes and teammate agency; record **all** attempts, not edited highlights.
**Systems:** [lab design](../01-design/combat-lab-01-advanced-playtest.md), [session capture](../03-tech/playtrace-01-session-capture.md).

## 1. Entry / automatic preparation

Tester opens the game's standard development/test launch (recording **on automatically**), selects `midfloor-lab-v1`, a fixed variant seed and one mode: Carl human + Donut AI; Donut human + Carl AI; or P1/P2 real local co-op. Any temporarily missing move/spell/enemy results in an explicitly rejected or reduced-capability test profile with a new ID; do not imply the advanced loadout is live before merge.

Before first simulated tick, program writes the session manifest and verified initial reset descriptor. Show small **REC** state in the lab UI or log; if disk storage is unavailable, explain `RECORDING FAILED — TEST NOT ML-ELIGIBLE` rather than silently continuing as though captured. Pause/options and 2P device ownership changes are data-bearing control events, not thrown away.

Include build/dirty state, exact flags, asset/layout hashes, camera, resolution, input device *class* (no hardware serial), player role, enabled mechanics, initial seed, intended mode and a pseudonym chosen by tester if desired. Time and runtime machine information are diagnostic, not policy input.

## 2. Repeated human sessions

| Trial | Mode / seed | Main question | Minimum evidence |
| --- | --- | --- | --- |
| A | Carl-human / Donut-AI, seed 1 | Baseline direct hit/contact, AI support, vertical navigation | Full raw recorder session, at least R01/R02/R05, one retry if an actual failure occurs |
| B | Donut-human / Carl-AI, same seed 1 | Can Donut actively fight and Carl help without stealing? | Complete session, mana/use/casting/assist attribution and camera notes |
| C | Real two-pad co-op, same seed 1 | Player equity and shared camera/traversal | Slot ownership and simultaneous action trace, no forced AI assumption |
| D | Carl-human or co-op, seeds 2 and 3 | Does the same arena support distinct strategy and temperament? | Three complete attempts of R05, finish causes, damage, reactions and qualitative notes |
| E | Paired A/B on same mode/seed | Does one targeted CF02, camera, enemy or level change improve feel? | Both exact rule configurations and action/event traces, no changes to other variables |

Seeds 1/2/3 mean three named **pre-pinned valid** configurations, not a promise that current main has gameplay RNG. If the environment seed does not affect simulation, the report must say so and show actual differences only when implemented. Separate development and held-out *geometry* variants for ML from repeated seeds on the same map.

A tester may jump directly to a room to avoid artificial repetition. Every retry is an episode **in the same session** and is retained even if unsuccessful. Two-player hardware not available? Mark Trial C **not tested**, not passed.

## 3. After each room (optional human judgement)

Fast, skippable ratings 1–5: basic attack enjoyment, visible contact consequences, dodge/reposition/vertical combat, enemy distinctiveness, companion/2P agency, camera readability, character tone, environmental payoff. Offer one-line annotation with tick range and named scene; tester can skip without losing recorded gameplay. The owner compares recordings with these judgements rather than treating entropy, automated win rate, score/viewers or spectacular kills as a fun oracle.

End-of-run report:
- Room clear/death/abandon, time in actual simulated ticks, attempts, damage, time airborne/stuck, move attempt/hit/whiff and accepted/rejected input breakdown.
- Enemy archetype, morale/AI events and genuinely different choices; relevant environmental kill causes and chain contacts with source attribution.
- Companion assists and player participation in 2P, solo swaps/commands, mana and unavailable special attempts.
- Camera pan/zoom spikes with recorded cause/hero positions, frame-time histogram/p50/p95/p99 on actual measured hardware.
- Replay status for session and each episode plus recorded vs expected ticks, corruption/missing flags and ML export eligibility.

For every claimed spectacle event, cross-check authoritative cause/event with a visible at-speed local capture **if a capture exists**; video is **not** an input to the dataset and is not required for recording. Do not count unobserved effects as failures in animation quality or as successful triggers.

## 4. Data integrity and ML acceptance ceremony

After one actual local session, run (CLI spelling is a design target, not a claim it exists yet):

```text
playtrace list
playtrace inspect <session_id>
playtrace verify <session_id> --fresh-process
playtrace export-transitions <session_id> --only-eligible
playtrace export-bc <session_id> --source human
```

Required report evidence (store artifact hashes, **not private human sessions in the public repository**):
1. Session manifest names correct repo commit, dirty flag, level+asset hash, ruleset, seed, mode and human/AI role; no hidden private data.
2. Actual tick count equals continuous input records; P1 and P2 actions persist including neutral inputs, holds and analog; hero effective inputs and ownership are correct after swaps.
3. Fresh-process replay reproduces all authoritative checkpoint state/event hashes and outcome. If it cannot, mark `replay_verified=false` and the exact first divergent tick.
4. For every eligible human row in the exported dataset, input `o_t` was observed **before** the action `a_t`; `o_(t+1)` occurred after; both have legal shapes and visibility masks. Exclude AI-controlled intervals from *human* demonstration datasets.
5. The exported episodes include failures, retries, neutral/missed/late actions, not only wins or spectacular kills. Sample and inspect a deterministic 20-transition subset; verify no secret enemy cooldown or off-camera world state is in the policy features.
6. Split by session/tester/geometry variant and prove there are no train/holdout transitions from a single session. Version the export and clearly link to source hashes.
7. Negative rehearsal: corrupt one action/checkpoint, remove one tick, simulate disk failure, force an interrupted run; verify appropriate *invalid / incomplete* outcomes rather than training silent garbage.

If snapshots cannot yet serialize the whole world, the report states `snapshot_support=reset_only`. The dataset may still be useful for behavioral cloning **if** actual time-aligned observations/actions are complete; world-state reconstruction beyond verified replay remains limited. A failed replay or missing mandatory observation **is never** described as verified/ML-ready.

## 5. Design iteration loop

Record one human issue with a source `session_id/episode_id/tick_range`, a plain-language symptom and expected consequence. Find the corresponding inputs, visible observation, event causal chain and video if separately available. Classify **mechanics / AI / level navigation / camera / animation+audio / controls / recorder**. Change one variable with versioned knobs and replay against matched seed/layout/controller/hardware. Compare verified objective outcomes and new human opinion; publish the resulting tune/decision to the project backlog. Preserve raw historical recordings unchanged for later offline analysis.

**Stop criteria:** no automatic recorder, ambiguous input ownership, broken fresh-process replay, invisible unavoidable threats, softlocked 2P progression, or corrupted schema requires a new playtest sample rather than speculative "ML feedback". No automatic gameplay-balancing commits from an ML score.
