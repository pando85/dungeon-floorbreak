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
| `SHA256SUMS` | checksums of the files above |

The GIFs are a native-pixel 960x540 crop of the 1920x1080 capture, one frame per tick, shown at 50 ms per tick (three times slower than real time so the freeze is visible), with the tick, Carl's move and the goblin's state printed on top.

## What the capture shows (measured, 60 ticks per second)

| Measure | Value |
| --- | --- |
| input to first damage | jab 3 ticks (50 ms); haymaker 5 ticks (83 ms) when pressed on a free Carl |
| the string | `cp01_single_goblin`: press on 136 inside the jab's window, the haymaker starts at once (jab tick 11, at least 7) and hits on 141. `cp01_haymaker_chain`: press on 141 (jab tick 3, frozen) starts the haymaker on 147 (jab tick 7) and hits on 152 |
| hit-stop | jab 3 ticks (Carl frozen 141..143, goblin 140..143), kill 8 ticks (Carl frozen 153..160, goblin 152..160); motion resumes on the first tick after |
| stagger | jab: staggered from 140, back to neutral on 153 (3 frozen + 10); haymaker 18 |
| knockback | the jab slides the goblin 1 px, the killing hit launches it from 430.8 to 482.3 (51.5 px): in the air 152..179, down 180..209, corpse from 210 and it stays |
| defeat confirmation | white flash (3 ticks) and a 3 px camera kick on the killing hit (2 px haymaker + 1), then the launch |
| goblin attack | tell on 127 (attack tick 0), smear 151..152, the impact opens on **153** (attack tick 26) and Carl is hurt on that tick; the blow freezes the goblin for 4 ticks like Carl, so attack ticks 27..29 run on 158..160, recovery 161..180, next tell on 222 (the 40-tick cooldown) |
| Carl's blow | 1 hit point, 18-tick flinch, 55 ticks of invulnerability, a 4-tick freeze for Carl and the goblin |
| cancelled attack | `cp01_interrupt`: tell 127, jab hit on 140 (attack tick 13): no swing, no damage, next tell on 175 (stagger over on 153, 20-tick cooldown) |
| retry | `cp01_retry`: Carl defeated on 400, restart 40 ticks later (event tick 439), the same attempt again, defeated on elapsed 839 |

## Comparison with the PR #45 baseline (COMBAT-INT-01)

Same scene, same inputs for `cp01_single_goblin`. The baseline files are kept in [`../combat-int-01`](../combat-int-01/README.md).

| | PR #45 baseline | CP-01 (this PR) |
| --- | --- | --- |
| jab hit | tick 125, damage and a 10-tick stagger, a 4 px push | tick 125 as before, plus a 3-tick freeze of Carl and the goblin, a white flash, a 1 px camera kick, a 10-tick stagger and a 1 px slide |
| second press (136) | a second jab, hit on 139, goblin defeated | inside the jab's window it is the haymaker: hit on 141 (5 ticks), 8-tick freeze, a 52 px launch |
| defeat | the goblin turns into a flat slab where it stood | launched away, 28 ticks of flight and a 30-tick lying phase, then a corpse that stays; no hurtbox, no collision |
| goblin attack | contact damage when the bodies overlap | a 24-tick tell (orange, club raised), a smear, a 4-tick active swing that only hurts in front, 20 ticks of recovery; interruptible in the tell |
| group | none | at most 2 attackers, tell starts at least 8 ticks apart |
| Carl hurt | 18-tick flinch | the same flinch, plus a 4-tick freeze, a turn towards the goblin and, on defeat, a 7-tick freeze, a knockdown launch and a retry 40 ticks later |
| Donut | hidden in fixtures | shown as the passive trail follower |
| state hash | `c6952df1e99c7c0a` | `e6537a1f17b0567b` (the changes are expected: new state and rules) |

## Fixtures and final hashes

| Fixture | Final `COMBAT_STATE` hash |
| --- | --- |
| `cp01_single_goblin` | `e6537a1f17b0567b` |
| `cp01_two_hits` | `e0663f9481b15b06` |
| `cp02_two_goblins` | `6e82b7de91397bea` |
| `cp01_goblin_attacks` | `a8aa111180066441` |
| `cp01_interrupt` | `5ef20dc28a509345` |
| `cp01_haymaker_chain` | `d48f327294242862` |
| `cp01_retry` | `ea023975d3500132` |

The story route is unchanged: `--scenario walk_right --capture-tick 250` still ends in `FLOOR1_SCENE_STATE ... hash=931868d6afa56498` (613 ticks: `82b8f87fa644de48`).

## Reproduce

From the repository root, after `bash scripts/export_assets.sh`:

```sh
cargo build --locked --bin floor01_scene
bash scripts/ci_floor01_combat.sh headless
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render
bash scripts/make_cp01_feel_evidence.sh
(cd docs/evidence/cp01-feel && sha256sum -c SHA256SUMS)
```

Provenance and rights: code by Claude Sonnet; no new art and no new third-party assets (placeholder rectangles drawn by the code); the characters and their art keep the recorded `ART-PUBLICATION-ALL-V1` scope, which is not a game distribution right.
