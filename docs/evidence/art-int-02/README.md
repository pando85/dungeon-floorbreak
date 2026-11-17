# ART-INT-02 evidence: Carl melee and goblin art in the Floor 1 combat fixtures

The approved Carl melee ([ANIM-CARL-02](../../02-art/carl-melee-anim-02-spec.md)) and goblin ([GOB-01](../../02-art/gob-01-goblin-spec.md), variants A and B) atlases replace the disclosed CP-01 placeholder rectangles in `floor01_scene`. Presentation only: the sim, every fixture `COMBAT_STATE` hash and the story-route hashes (`walk_right` t250 `931868d6afa56498`, t613 `82b8f87fa644de48`) are unchanged. Design and mapping: [Combat animation](../../03-tech/floor01-scene.md#combat-animation-art-int-02). **Nobody has judged the look or the feel yet; the owner review is pending.**

Everything below comes from the real Bevy render (software Vulkan under Xvfb, 1920x1080, scale 3, lighting on), one capture per tick, produced by `xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render`; the render run's final sim state and the frames it drew (`COMBAT_ANIM_STATE`) are compared with the headless replay by that script. `python3 scripts/make_art_int02_evidence.py` builds the images from the captures.

## GIFs

Native-pixel 960x540 crops of the 1920x1080 captures (no scaling), one frame per tick shown for 50 ms (3x slower than real time), captioned with the tick, Carl's drawn frame and move clock (`!` = frozen by hit-stop) and every goblin's drawn frame. A full 1920x1080 GIF would be several MB per fixture, so the key ticks are also committed as full 1920x1080 stills.

| File | Shows |
| --- | --- |
| `cp01_haymaker_chain_ticks128-188_960x540.gif` | goblin tell, Carl's jab (a1 on the hit tick 140, recoil held), the haymaker (b1 on tick 152) killing it: recoil through the 9-tick hit-stop, launch frame, flight |
| `cp01_goblin_attacks_ticks128-188_960x540.gif` | the goblin's tell, smear, impact on Carl (tick 153), Carl's flinch and the goblin's recovery |
| `cp01_interrupt_ticks122-182_960x540.gif` | a jab landing inside the tell: the swing never happens, recoil for 4 ticks then the stagger frame held |
| `cp02_two_goblins_ticks60-120_960x540.gif` | variants A and B walking in, jab, haymaker kill of the first goblin and the second one caught (non-lethal haymaker: recoil 5 + 1 ticks, stagger frame) |

## Per-tick strips

* `strip_haymaker_kill_ticks150-161.png`: every tick from the startup of the haymaker through the kill. Tick 152 is the hit (b1 contact, goblin recoil 16, hit-stop 8), ticks 153 to 160 hold both frames while the sim is frozen, tick 161 is the first moving tick (launch frame 18).
* `strip_goblin_swing_hits_carl_ticks149-160.png`: wind-up hold (11), smear (12, ticks 151 and 152), impact (13) on the hit tick 153, Carl's impact frame (15) held through the 4-tick freeze and the following flinch ticks; the goblin is drawn over Carl while it swings.

## Stills (full 1920x1080)

`still_jab_contact_a1_tick140`, `still_haymaker_kill_contact_b1_recoil_tick152`, `still_kill_flight_airborne_frame_tick168`, `still_goblin_tell_tick140`, `still_goblin_impact_on_carl_tick153`, `still_jab_interrupts_the_tell_recoil_held_tick143`, and `still_goblins_A_B_with_carl_donut_x2.png` (variants A and B next to Carl and Donut, a 2x crop of the 1920x1080 capture).

## Traces

`anim_trace_cp01_haymaker_chain.txt`, `anim_trace_cp01_goblin_attacks.txt`, `anim_trace_cp02_two_goblins.txt`: one `COMBAT_ANIM_TRACE` row per tick (`floor01_scene --headless-floor01-smoke --fixture <name> --anim-trace`): Carl's move and clock, hurt and hit-stop counters, each goblin's mode, attack tick, hit-stop and height, and the atlas frame drawn. CI regenerates them and diffs against these files (`scripts/ci_floor01_combat.sh headless`).

## Not shown (no sim state yet)

The goblin `knockback_contact` frame and the get-up frames of Carl and the goblin: nothing in the CP-01 sim produces them (CP-02). They are mapped and unit tested, not wired.

## Rights and provenance

Fan-concept art under the recorded `ART-PUBLICATION-ALL-V1` scope ([rights register](../../04-production/rights-and-release.md)); not a licence to distribute a playable game. Art: Carl melee and goblin designed by Claude Opus, executed by Claude Sonnet, reviewed by a separate Claude Opus (see their own evidence). This integration (Bevy adapter, frame selection, tests, evidence) was done on the default tier (Claude Sonnet); no art was drawn or changed.

`SHA256SUMS` lists every file in this folder except this README.
