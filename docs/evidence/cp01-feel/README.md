# CP-01 evidence: goblin takedown feel

Real Bevy captures, per-tick traces and deterministic logs of the CP-01 feel pass ([scene doc](../../03-tech/floor01-scene.md#combat-fixtures-combat-int-01-cp-01)). **Mechanical probes only**: the goblin is a disclosed placeholder rectangle (magenta band on its head) and Carl's strike is a small pale box, because the GOB-01 and ANIM-CARL-02 art have not merged. Nothing here is a feel verdict or an art approval: **a human must play it** (issue #40 stays open). All numbers are starting values from `src/feel.rs`.

Stills and GIFs are screenshots of the real Bevy window (lighting on, 1920x1080 = 640x360 x 3, Xvfb with software Vulkan on the author's machine). They are not reproduced bit for bit by CI (rendering varies by driver); the event logs and hashes are: CI diffs the headless replays against `headless_*.txt` and requires the real render loop to print the same events and final state for every fixture.

| File | What it shows |
| --- | --- |
| `cp01_single_goblin_ticks118-178_960x540.gif` | the baseline inputs (jab on 122, second press on 136): the jab hit with its 3-tick freeze and the goblin's flash, stagger, the second press as the haymaker, the killing hit with an 8-tick freeze, the launch and the landing |
| `cp01_haymaker_chain_ticks128-188_960x540.gif` | the goblin's tell (orange, club raised), the jab hit inside it (the swing is cancelled), the string into the haymaker, the kill |
| `cp01_interrupt_ticks122-182_960x540.gif` | the tell, the jab on attack tick 13, the stagger and the goblin walking away from the cancelled swing |
| `strip_haymaker_kill_ticks150-161.png` | **every tick around the killing hit** (152): Carl's move clock stands still on `haymaker:5` and the goblin stays in its flash pose for the whole hit-stop (ticks 153 to 160), both resume on 161 |
| `timing_haymaker_chain.png` | input, startup, active, hit-stop, stagger, knockback, defeat and camera kick on one time axis (real tick data) |
| `timing_interrupt.png` | the same lanes for the interrupted tell: the next tell waits for the stagger and the 20-tick post-hurt cooldown |
| `timeline_goblin_attack.png` | the goblin's tell, swing and recovery against Carl's 18-tick flinch, 55 ticks of invulnerability and hit points (`cp01_goblin_attacks`) |
| `still_*_1920x1080.png` | full captures: the jab interrupting the tell (140), the killing hit's hit-stop (152), the launch (175), the baseline kill (141), a tell (130) |
| `trace_<fixture>.txt` | one `COMBAT_TRACE` row per tick (`--combat-trace`): Carl's move and tick (`!` = frozen), positions, health, flinch, i-frames, camera kick, goblin mode, attack tick, hit-stop |
| `headless_<fixture>.txt` | every `COMBAT_EVENT` and the final `COMBAT_STATE` hash line of the seven fixtures |
| `SHA256SUMS` | SHA-256 checksums of every file in this folder except this README and `SHA256SUMS` itself (`sha256sum -c SHA256SUMS`) |

The GIFs are a native-pixel 960x540 crop of the 1920x1080 capture, one frame per tick, shown at 50 ms per tick (three times slower than real time so the freeze is visible), with the tick, Carl's move and the goblin's state printed on top.

## What the capture shows (measured, 60 ticks per second)

| Measure | Value |
| --- | --- |
| input to first damage | jab 3 ticks (50 ms); haymaker 5 ticks (83 ms) when pressed on a free Carl |
| the string | `cp01_single_goblin`: press on 136 inside the jab's window, the haymaker starts at once (jab tick 11, at least 7) and hits on 141. `cp01_haymaker_chain`: press on 141 (jab tick 3, frozen) starts the haymaker on 147 (jab tick 7) and hits on 152; Carl steps forward 1 px per tick on 148 to 151 (400 to 404) |
| hit-stop | jab 3 ticks (Carl frozen 141..143, goblin 140..143), kill 8 ticks (Carl frozen 153..160, goblin 152..160); motion resumes on the first tick after |
| stagger | jab: staggered from 140, back to neutral on 153 (3 frozen + 10); haymaker 18 |
| knockback | the jab slides the goblin 3 px (429.8 to 432.8), the killing hit launches it from 432.8 to 506.1 (73 px): in the air 152..182 (9 of those ticks are the hit-stop), down 183..212, corpse from 213 and it stays |
| defeat confirmation | white flash (3 ticks) and a 3 px camera kick on the killing hit (2 px haymaker + 1; none on the jab), then the launch |
| goblin attack | tell on 127 (attack tick 0), smear 151..152, the impact opens on **153** (attack tick 26) and Carl is hurt on that tick; the blow freezes the goblin for 4 ticks like Carl, so attack ticks 27..29 run on 158..160, recovery 161..180, next tell on 222 (the 40-tick cooldown) |
| Carl's blow | 1 hit point, 18-tick flinch, 55 ticks of invulnerability, a 4-tick freeze for Carl and the goblin |
| cancelled attack | `cp01_interrupt`: tell 127, jab hit on 140 (attack tick 13): no swing, no damage, next tell on 177 (stagger over on 153, 20-tick cooldown, and the 3 px push to walk back) |
| retry | `cp01_retry`: Carl defeated on 400, restart 50 ticks later (event tick 449), the same attempt again, defeated on elapsed 849 |

## Camera kick direction (review round 1, blocker B1)

The first version moved the view toward the attacker, so the whole scene jolted against the knockback. The kick now moves the view against the hit direction: the scene jolts the way the goblin flies. The goblin's screen x (world x minus the view's left edge, kick included; from `trace_cp01_haymaker_chain.txt`, `cam_left` and `kick` columns) never falls behind its pre-hit value while it is frozen:

| tick | event | kick | view left | goblin screen x |
| --- | --- | --- | --- | --- |
| 139 | before the jab hit | 0 | 104 | 325.8 |
| 140 to 143 | jab hit and its hit-stop (no kick for a jab) | 0 | 104 | 325.8 |
| 151 | before the kill | 0 | 104 | 328.8 |
| 152 | haymaker kill | -3 | 101 | 331.8 |
| 153 to 154 | hit-stop | -3, -2 | 101, 102 | 331.8, 330.8 |
| 155 to 158 | hit-stop | -2, -2, -1, -1 | 102 to 103 | 330.8, 330.8, 329.8, 329.8 |
| 159 to 160 | hit-stop ends | 0 | 104 | 328.8 |
| 161 | the launch starts | 0 | 104 | 331.5 |

The camera and Donut's trail are frozen during the hit-stop (`SceneSim::hold_tick`), so the view's left edge stays 104 apart from the kick.

## Comparison with the PR #45 baseline (COMBAT-INT-01)

Same scene, same inputs for `cp01_single_goblin`. The baseline files are kept in [`../combat-int-01`](../combat-int-01/README.md).

| | PR #45 baseline | CP-01 (this PR) |
| --- | --- | --- |
| jab hit | tick 125, damage and a 10-tick stagger, a 4 px push | tick 125 as before, plus a 3-tick freeze of Carl and the goblin, a white flash (no camera kick for a jab), a 10-tick stagger and a 3 px slide |
| second press (136) | a second jab, hit on 139, goblin defeated | inside the jab's window it is the haymaker: hit on 141 (5 ticks), 8-tick freeze, a 73 px launch |
| defeat | the goblin turns into a flat slab where it stood | launched away, 28 ticks of flight and a 30-tick lying phase, then a corpse that stays; no hurtbox, no collision |
| goblin attack | contact damage when the bodies overlap | a 24-tick tell (orange, club raised), a smear, a 4-tick active swing that only hurts in front, 20 ticks of recovery; interruptible in the tell |
| group | none | at most 2 attackers, tell starts at least 12 ticks apart |
| Carl hurt | 18-tick flinch | the same flinch, plus a 4-tick freeze, a turn towards the goblin and, on defeat, a 7-tick freeze, a knockdown launch and a retry 50 ticks later |
| Donut | hidden in fixtures | shown as the passive trail follower |
| state hash | `c6952df1e99c7c0a` | `cbb307e8a1a7420f` (the changes are expected: new state and rules) |

## Fixtures and final hashes

| Fixture | Final `COMBAT_STATE` hash |
| --- | --- |
| `cp01_single_goblin` | `cbb307e8a1a7420f` |
| `cp01_two_hits` | `119a431dad6c7d0b` |
| `cp02_two_goblins` | `4f3b150ac8a0c633` |
| `cp01_goblin_attacks` | `a8aa111180066441` |
| `cp01_interrupt` | `ee81fa34f09491e0` |
| `cp01_haymaker_chain` | `01a7fa7a9770271d` |
| `cp01_retry` | `edaa118cf0e31e94` |

The story route is unchanged: `--scenario walk_right --capture-tick 250` still ends in `FLOOR1_SCENE_STATE ... hash=931868d6afa56498` (613 ticks: `82b8f87fa644de48`).

## Reproduce

From the repository root, after `bash scripts/export_assets.sh`:

```sh
cargo build --locked --bin floor01_scene
bash scripts/ci_floor01_combat.sh headless
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render-sequences
bash scripts/make_cp01_feel_evidence.sh
(cd docs/evidence/cp01-feel && sha256sum -c SHA256SUMS)
```

Provenance and rights: code by Claude Sonnet; no new art and no new third-party assets (placeholder rectangles drawn by the code); the characters and their art keep the recorded `ART-PUBLICATION-ALL-V1` scope, which is not a game distribution right.

## DEATH-B note

`headless_cp01_kick.txt` gained ` force=heavy` on its `enemy_defeated` line (the lethal second kick launches, so its force tier is heavy). The `COMBAT_STATE` hash is unchanged; `SHA256SUMS` was updated. See [species death profiles](../../03-tech/species-death-profiles.md). New fixture `cp03_kick_block` (kick, then the block crushes the launched goblin; the line ends `instigator=0`): `headless_cp03_kick_block.txt`, final hash `73e409161ce5434f`, a new fixture so it is not a migration.

## INSTIGATOR-WINDOW note

`headless_cp03_kick_block.txt` lost ` instigator=0` on its `enemy_defeated` line and its `COMBAT_STATE` hash is now `ab324fe393e5f1b6` (was `73e409161ce5434f`): the kicked goblin gets up (about tick 80) before the block lands (tick 179), and `Goblin::hit_by` is now cleared when the goblin is `Neutral` again. `SHA256SUMS` was updated. See [species death profiles](../../03-tech/species-death-profiles.md).

## DEATH-E1 note

New fixture `cp03_goblin_pileup` (a presenter stress scenario, not a gameplay design): ten stationary goblins defeated by mixed causes within 65 ticks (three jab defeats on tick 83, four shore-block crushes on 139, three lethal kicks on 148). `headless_cp03_goblin_pileup.txt`, final hash `6a82811d8f31c475`, a new fixture so it is not a migration; no existing fixture, log or hash changed. The fixture exposed a pre-existing presentation bug, recorded as **GOB-STACK-01** in the backlog (a nudged stacked goblin drawn with the fist not touching it, a 6 px nudge pop when stacked goblins are launched, left-facing blows landing 40 to 60 px deep); the two animation tests skip exactly those cases for this fixture and assert everything else. See [DEATH-E1](../death-e/README.md).

## DEATH-D note

Two new fixtures, so no migration: `cp03_block_edge_decap` (the `cp03_shore_kill` setup with the goblin on the footprint's right edge facing the block; the landing line is `cause=shore_block_edge credit_player=0`): `headless_cp03_block_edge_decap.txt`, final hash `13d98f79028217e9`; and `qa_blast_breakup` (a QA-only blast charge at x 520 on tick 60 with three stationary goblins: point blank, ring and untouched; lines `kind=blast charge=0` and `cause=explosion charge=0 point_blank`): `headless_qa_blast_breakup.txt`, final hash `c3d2b3703f495284`. All 13 earlier logs and hashes are byte-identical (the CI headless job diffs every one). See [DEATH-D](../../03-tech/floor01-scene.md#death-d-decapitation-and-explosion-finishes) and [evidence](../death-d/README.md).
