# CF02-C2 + CF02-F evidence

One grouped PR (`feat/cf02-c2f`): C2 (solo swap and Donut's bail-out) lands first, then F (the microarena). Replays of every fixture are in [`../replays/`](../replays/) (run twice by `scripts/ci_floor01_combat.sh headless` and pinned by the `GOLDENS` of `src/donut_fixtures.rs` and `src/cf02_fixtures.rs`). Tick numbers are attempt-local ticks of the committed replays.

## CF02-C2

| Beat (design question) | Fixture or test | What it shows |
| --- | --- | --- |
| Swap pan eased in and out (Q20) | [`pan_trace.md`](pan_trace.md), test `cf02_swap_pan_trace_eases_in_and_out_and_arrives_where_the_legacy_pan_does` | legacy: 8 px on the swap tick, 19 equal steps, a hard stop; eased: 2, 4, 6, 8, 10, 12 px, a cruise, a soft arrival, the same arrival column |
| Swap cooldown 12 (Q19) | `cf02_swap_eased_pan`, test `cf02_swap_cooldown_is_12_and_a_press_the_legacy_one_refuses_swaps` | the press the legacy 20-tick cooldown lets expire swaps at tick 62; the buffer (6), the hurt block and 2P ownership are unchanged |
| Llama fair fire under the pan (Q20, Q40) | `cf02_swap_eased_pan`, test `a_llama_on_the_view_edge_fires_only_while_inside_the_pan_moved_view` | a tell starts only while the whole body is inside the simulation view of that tick, in the legacy pan and in the eased one |
| Donut's bail-out (Q55, C6) | `cf02_donut_bailout`, [`gifs/donut_bailout_1x.gif`](gifs/donut_bailout_1x.gif) | swat press on 5, Jump on 12 (swat tick 7): a cat hop of vx 170 px/s away and vy 300 (apex about 43 px, about 33 air ticks); a swat pressed in the air never starts, the one after the 2-tick landing recovery does; the pounce recovery (touchdown + 8) hops the same way |
| 2P and legacy unchanged | tests `the_cf02_swap_tuning_changes_nothing_in_two_player`, `the_bailout_is_cf02_only_and_the_ai_never_bails_out` | three 2P fixtures keep their hash with the `--cf02` swap tuning; outside `cf02_*` the recovery jump is the legacy jump; the AI Donut never bails out |

## CF02-F: the microarena

`floor01_scene --cf02` starts in the room (the flag is the whole launch; add `--scale 3` for a big window, `--deck` for 640x400, `--autopilot` to watch the stand-in play it, `--cf02-seed N` for another temperament draw). Captures are real Bevy frames (`scripts/cf02_arena_captures.sh`):

| 640x360 | 640x400 | 1920x1080 |
| --- | --- | --- |
| [t40](captures/arena_t40_640x360.png), [t330](captures/arena_t330_640x360.png) | [t40](captures/arena_t40_640x400.png), [t330](captures/arena_t330_640x400.png) | [t40](captures/arena_t40_1920x1080.png), [t330](captures/arena_t330_1920x1080.png) |

Tick 40 shows the left end, the spawn and the rotten shore; tick 330 is the automatic camera at its widest (a llama has taken aim). No frame shows void. 1x GIFs: [Donut bail-out](gifs/donut_bailout_1x.gif), [a shore drop](gifs/shore_drop_1x.gif) (the block lands on Carl, who stood in the footprint), [a goblin self-own](gifs/goblin_self_own_1x.gif), [a perch dive](gifs/perch_dive_1x.gif), [a 1P swap in the room](gifs/solo_swap_1x.gif) (`cf02_arena_swap`: Carl runs, the player swaps to Donut and back; the trailing bot stays inside the 40 px camera deadzone, so the camera follows without a long pan) and [the 1P assistant](gifs/solo_assist_1x.gif) (`cf02_arena_direct`: the AI Donut's swats on a llama tell and a goblin tell).

| Beat (design question) | Fixture | What the replay shows |
| --- | --- | --- |
| Direct (Q32, Q39) | `cf02_arena_direct` | the room is completed solo by the direct plan on tick 745 with no death; the AI Donut starts 3 assist moves (swats on 362, 422, 552) and lands 3 of the 16 hits (18.8 %, floor: at least 3 starts, at most 25 %) |
| Environmental (Q31, Q32) | `cf02_arena_environment` | Carl strikes the shore once (323), one goblin swing trips it (`shore_tripped goblin=0 hits=2 broke=0 credited=0`, 381), the next swing breaks it (`hits=3 broke=1`, 473) and the block lands (509). Since review follow-up U4 the stand-in plan completes the room by this route (no deaths, last enemy at 814; it used to die twice) |
| 1P swap (Q19) | `cf02_arena_swap` | the solo swap happens twice in the room (the second press is the one the 12-tick cooldown lets through) |
| Cooperation (Q32) | `cf02_arena_coop` | two players: P2 Donut swats the tell aimed at Carl and Carl finishes; the room is complete on 597 |
| Three attempts (Q32, Q58) | `cf02_arena_three_attempts` | Carl is defeated three times in front of the first wave; each restart draws its temperaments again from the bag (two brawlers, two cowards, one show-off) and the reactions differ per attempt |
| The block hurts the careless striker (Q30, Q42) | `cf02_block_hurts_carl` | three jabs from x 412 facing left; on 94 the block lands: `hero_hurt source=21320 health_left=3`, the flinch and a knock away from the block |
| ... and spares the left striker (Q30) | `cf02_block_spares_left_striker` | the same from x 365 facing right: the block lands 25 px away, nobody is hurt |
| ... and the partner in 2P (Q42) | `cf02_block_hurts_partner` | P2 Donut stands in the footprint: she takes the 1 damage, Carl none |
| Goblin self-own (Q31, Q41) | `cf02_goblin_self_own` | the shore starts at 2 hits; the chasing goblin's swing is the third: `shore_tripped goblin=0 hits=3 broke=1` (never `shore_hit` or `shore_broken`), the block crushes the second goblin on 94; one swing is one trip (test `one_goblin_swing_trips_the_shore_once`, with and without the hit-stop of the hurt); no CINE moment (tested through the adapter) |
| Intact shore ignores goblins (Q31) | `cf02_intact_shore_ignores_goblins` | the same scene with an intact shore: swings pass by, nothing trips |
| Parapet cover (Q28) | `cf02_parapet_cover` | two spits, no hurt behind the 48 px parapet; the control (Carl 20 px to the right of it) is hurt |
| Perch climb (Q45) | `cf02_perch_slab_climb` | a jump from x 490 lands on the 80 px slab, he walks along it |
| Perch above the spit (Q28, Q45) | `cf02_perch_above_spit` | two spits pass under him on the right perch, no hurt |
| Perch dive (Q52, Q45) | `cf02_perch_dive` | steps off the left perch and dives onto the goblin below: one `dive_stomp` hit |

Also tested: the solo and 2P autopilots complete the room in every shore state (intact, 1 hit, 2 hits, broken with the block landed); both heroes can run the lane end to end; the scroll camera and the automatic camera (16:9 and 16:10) stay inside the room for a whole autopilot run; the whole render bounds are drawn.

## Checks run

- `cargo fmt --all -- --check`, `cargo test --locked --all-targets`, `bash scripts/ci_floor01_combat.sh headless` (all `cf02_*` replays run twice and equal to the committed ones, every legacy golden and probe unchanged), `scripts/check_playtrace_isolation.sh`, `python3 scripts/check_docs.py`. `cargo clippy --locked --all-targets -- -D warnings` reports only the pre-existing `needless_range_loop` of untouched lines (local toolchain 1.99.1).
- The real render loop equals the headless replay: [`render_match.txt`](render_match.txt) (`scripts/cf02_render_match.sh`): 16 fixtures (C2 and F) and the live `--cf02` room under the autopilot.
- Live smoke: `xvfb-run floor01_scene --cf02 --scale 1 --exit-after-ticks 900` (idle player, and with `--autopilot`) completes without a panic and the autopilot logs `CF02_ARENA_COMPLETE tick=487`. Both recordings (`--record`) verify with `playtrace verify` in a fresh process (`result=PASS scope=full build=identical`) after the verifier was taught the room, the room rules and the `--cf02` swap tuning (a first run without that failed `content_mismatch`, a real PLAYTRACE finding of this PR; the regression test is `a_cf02_room_session_verifies_from_its_reset_descriptor`, which fails on the pre-fix verifier).

**NOT RUN:** a real two-pad session, a human play of the room, frame-time on real hardware, audio listening. The GIFs show the committed replays, not a feel verdict.

Rights: the frames show only the committed original Carl, Donut, goblin and llama art and the approved ENV-01/02 kit pieces under the recorded scope in the [rights register](../../../04-production/rights-and-release.md). No new art. Model attribution: Claude Sonnet (this PR).
