# ART-INT-02 evidence: Carl melee and goblin art in the Floor 1 combat fixtures

> **Predates FA-Q38 (i-frame cue) and Donut's hurt frames.** Captured before both heroes flicker during their post-hit invulnerability and before Donut's ANIM-DONUT-02 hurt, knockback and knockdown frames were wired (PR #259; see [f01-donut-hurt](../f01-donut-hurt/README.md)). Affected here: `cp01_goblin_attacks_ticks128-188_960x540.gif` and `cp01_retry_ticks398-449_960x540.gif` (Carl is hurt in them). Kept as captured, not regenerated; a regeneration would show the cue.

The approved Carl melee ([ANIM-CARL-02](../../02-art/carl-melee-anim-02-spec.md)) and goblin ([GOB-01](../../02-art/gob-01-goblin-spec.md), variants A and B) atlases replace the disclosed CP-01 placeholder rectangles in `floor01_scene`. Presentation only: the sim, every fixture `COMBAT_STATE` hash and the story-route hashes (`walk_right` t250 `931868d6afa56498`, t613 `82b8f87fa644de48`) are unchanged; the one new hash was the `cp01_kick` fixture (`13a31642299f0768`, input data only). **Updated by I-03 PR A:** the kick now launches, `cp01_kick` is retimed (second kick on tick 247, 340 ticks) and its hash is `9e13d64619149b71`; the kick GIF and strip below were regenerated from a real xvfb render of the new fixture. Design and mapping: [Combat animation](../../03-tech/floor01-scene.md#combat-animation-art-int-02). **Nobody has judged the look or the feel yet; the owner review is pending.**

Everything below comes from the real Bevy render (software Vulkan under Xvfb, 1920x1080, scale 3, lighting on), one capture per tick, produced by `xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render-sequences` (with no arguments it captures every sequence; `render` only runs the live-loop match); the render run's final sim state and the frames it drew (`COMBAT_ANIM_STATE`) are compared with the headless replay by that script. `python3 scripts/make_art_int02_evidence.py` builds the images from the captures.

**Hit-stop counters.** A trace shows the goblin's hit-stop counter *after* the hit tick, so a haymaker kill shows 8 and the goblin is frozen for 9 ticks (the hit tick and 8 more); the jab's 3 is 4 frozen ticks, the kick's 8 is 9, the kill by a kick (8 + 3) is 12. The recoil frame is shown for every frozen tick.

## Review fix round 1

* **B1, contact in hit-stop:** `render_contact_jab_haymaker_kick_x3.png` shows a1, b1 and k3 on their hit ticks with the goblin drawn toward Carl. The striking limb overlaps the goblin by 2 px (jab and haymaker) and 4 px (kick, drawn back 1 px from 7) on every hit-stop tick of every fixture (shifts: jab 3 to 8 px, haymaker 1 to 5 px). The trace columns `g0_dx` (drawn shift) and `g0_ovl` (overlap in pixels) show it per tick.
* **F-a, stacked goblins:** `still_retry_goblins_drawn_apart_x2.png` (cp01_retry ticks 399, 416, 430): the second goblin is drawn 6 px away from Carl and behind the first one.
* **F-b:** goblins A and B walk three frames out of phase (`cp02_two_goblins` GIF).
* **F-c:** `strip_carl_knockdown_ticks399-448.png`: the blow (h0 held through the freeze), launch, flight, landing (d0) and sprawl (d1) without a head drop or a pop.
* **F-d:** `strip_kick_ticks133-148.png` and `cp01_kick_ticks128-188_960x540.gif`: the new `cp01_kick` fixture, k3 on the hit tick 135 (recoil for the 9 frozen ticks, then the stagger frame) and the killing kick on 183. Since I-03 PR A the kick launches the goblin (recoil frame 16 for the hit-stop, then `knockback` 18, 19, `down` and `getup` by tick 188) and the killing kick lands on tick 260, outside the 61-tick window. The GIF and strip show this (regenerated with `make_art_int02_evidence.py` from a `--scale 3` capture of ticks 128 to 188).
* **F-e:** lying goblins are drawn below Donut.

## GIFs

Native-pixel 960x540 crops of the 1920x1080 captures (no scaling), one frame per tick shown for 50 ms (3x slower than real time), captioned with the tick, Carl's drawn frame and move clock (`!` = frozen by hit-stop) and every goblin's drawn frame. A full 1920x1080 GIF would be several MB per fixture, so the key ticks are also committed as full 1920x1080 stills.

| File | Shows |
| --- | --- |
| `cp01_haymaker_chain_ticks128-188_960x540.gif` | goblin tell, Carl's jab (a1 on the hit tick 140, recoil held), the haymaker (b1 on tick 152) killing it: recoil through the 9 frozen ticks, launch frame, flight |
| `cp01_goblin_attacks_ticks128-188_960x540.gif` | the goblin's tell, smear, impact on Carl (tick 153), Carl's flinch and the goblin's recovery |
| `cp01_interrupt_ticks122-182_960x540.gif` | a jab landing inside the tell: the swing never happens, recoil for 4 ticks then the stagger frame held |
| `cp02_two_goblins_ticks60-120_960x540.gif` | variants A and B walking in (out of phase), jab, haymaker kill of the first goblin and the second one caught (non-lethal haymaker: recoil for 6 frozen ticks, then the stagger frame) |
| `cp01_kick_ticks128-188_960x540.gif` | the push kick: k3 on the hit tick 135 inside the tell, then the launch, flight, lying and get-up of the kicked goblin (ticks 128 to 188; the killing kick on 260 is outside the window) |
| `cp01_retry_ticks398-449_960x540.gif` | Carl's defeat, launch, flight, landing and sprawl until the restart, the two goblins drawn apart |

## Per-tick strips

* `strip_haymaker_kill_ticks150-161.png`: every tick from the startup of the haymaker through the kill. Tick 152 is the hit (b1 contact, goblin recoil 16, hit-stop counter 8), ticks 153 to 160 hold both frames while the sim is frozen, tick 161 is the first moving tick (launch frame 18).
* `strip_goblin_swing_hits_carl_ticks149-160.png`: wind-up hold (11), smear (12, ticks 151 and 152), impact (13) on the hit tick 153, Carl's impact frame (15) held through the 4-tick freeze and the following flinch ticks; the goblin is drawn over Carl while it swings.
* `strip_carl_knockdown_ticks399-448.png`, `strip_kick_ticks133-148.png`: see the fix round above.

## Stills (full 1920x1080)

`still_jab_contact_a1_tick140`, `still_haymaker_kill_contact_b1_recoil_tick152`, `still_kill_flight_airborne_frame_tick168`, `still_goblin_tell_tick140`, `still_goblin_impact_on_carl_tick153`, `still_jab_interrupts_the_tell_recoil_held_tick143`, and `still_goblins_A_B_with_carl_donut_x2.png` (variants A and B next to Carl and Donut, a 2x crop of the 1920x1080 capture).

## Traces

`anim_trace_cp01_haymaker_chain.txt`, `anim_trace_cp01_goblin_attacks.txt`, `anim_trace_cp02_two_goblins.txt`, `anim_trace_cp01_kick.txt`: one `COMBAT_ANIM_TRACE` row per tick (`floor01_scene --headless-floor01-smoke --fixture <name> --anim-trace`): Carl's move and clock, hurt and hit-stop counters, each goblin's mode, attack tick, hit-stop, height, drawn shift (`_dx`), strike overlap (`_ovl`) and the atlas frame drawn. CI regenerates them and diffs against these files (`scripts/ci_floor01_combat.sh headless`). The committed event log of the new fixture is `../cp01-feel/headless_cp01_kick.txt`.

## Not shown (no sim state yet)

The goblin `knockback_contact` frame and the get-up frames of Carl and the goblin: nothing in the CP-01 sim produces them (CP-02). They are mapped and unit tested, not wired.

## Rights and provenance

Fan-concept art under the recorded `ART-PUBLICATION-ALL-V1` scope ([rights register](../../04-production/rights-and-release.md)); not a licence to distribute a playable game. Art: Carl melee and goblin designed by Claude Opus, executed by Claude Sonnet, reviewed by a separate Claude Opus (see their own evidence). This integration (Bevy adapter, frame selection, tests, evidence) was done on the default tier (Claude Sonnet); no art was drawn or changed.

`SHA256SUMS` lists every file in this folder except this README.
