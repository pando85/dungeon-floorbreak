# ACT1 evidence: the chase, the shore and the guild at the start of the floor run

The first three rooms of the fifteen-room floor run: the chase, `act1_chase` ("The First Chase", ACT1-B, the Murder Dozer opening), the shore, `act1_shore` ("The Rotten Shore"), and the guild, `act1_guild` ("The Guild Door", GUILD-BEAT-01, Donut's biscuit, ACT1-A). All launch with `--scene f01_late --f01-room ID` and are walked by `--f01-run` before the corral. They are **labelled playable greybox** (flat decor rectangles on the existing kit, the guide is the existing `mordecai_h44` sprite, the chase's machine is room 3's approved DOZ-01 rig; the ENV-02 dressing is ACT1-Q7, audio is ACT1-Q2). Design: [floor-01-act-01-integration.md](../../01-design/floor-01-act-01-integration.md) (section 17 holds the ACT1-IMP defaults). The stills and readings of the shore and the guild are ACT1-A's (captured on its head); the chase files below are ACT1-B's. Owner decision of 2026-11-25 ("Go with all because I could not test it until next week" and "Ensure that everything ends on main for testing all together next week"): everything on by default, reachable in story order. Game feel is the owner's call: nothing here is accepted.

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_shore_spawn_*.png` | The shore at spawn: the wedged machine decor (flat dark rectangle) at the left edge | window, `--f01-room act1_shore --act1-pilot strike`, tick 20 |
| `still_shore_strike_beam_*.png` | Carl at the strike column (local 348), the rotten beam after the first hit, the first goblin walking in | same, tick 100 |
| `still_shore_block_falling_*.png` | The beam broken, the block falling over the goblin that followed Carl (footprint local 392..472) | same, tick 196 |
| `still_shore_block_kill_*.png` | The block landed; the goblin under it is defeated (credited to the block) | same, tick 212 |
| `still_guild_door_closed_*.png` | Donut before the zone: the prompt reads `SWAT`, the `guild_door` stands (brown column), Mordecai as the guide (`mordecai_h44`, x 380, facing left) | window, `--f01-room act1_guild --act1-pilot donut`, tick 60 |
| `still_guild_missile_notice_*.png` | Donut in the zone: the `MAGIC MISSILE` notice and the prompt changed to `MISSILE` (the biscuit, tick 77) | same, tick 92 |
| `still_guild_door_open_*.png` | The door gone after the unlock (open event in tick 77, not colliding from tick 78) | same, tick 112 |
| `still_chase_spawn_*.png` | S1 at spawn: the false calm, the timber decor and the soffit shapes, no machine | window, `--f01-room act1_chase --act1-pilot hide`, tick 20 |
| `still_chase_rumble_*.png` | The machine's rumble: Carl holds just past the trigger, the window decor at the left edge | window, `--act1-pilot watch`, tick 380 |
| `still_chase_emergence_*.png` | The emergence at the window: the DOZ-01 rig, no riders (ACT1-AD9), the wheel front at 888 | same, tick 470 |
| `still_chase_timber_smash_*.png` | Timber set A smashed by the front (the ENV-02 splinter burst; the decor is hidden) | `--act1-pilot hide`, tick 510 |
| `still_chase_alcove_before_tell_*.png` | The lit `alcove`, the brightest shape of the lane, ahead of the heroes before the tell | same, tick 600 |
| `still_chase_tell_*.png` | The tell (ticks 747 to 783): the machine at the cruise, both heroes in the alcove | same, tick 765 |
| `still_chase_pocket_wheel_passing_*.png` | Both heroes in the pocket with the wheel passing | same, tick 825 |
| `still_chase_barricade_impact_*.png` | The barricade impact (tick 873): the barricade is gone, splinters at the front | same, tick 880 |
| `still_chase_wedge_*.png` | The greybox wedge: the rig clipped to the arch's right edge and darkened, the exit door beyond (console label `GREYBOX WEDGE`) | same, tick 1330 |
| `still_chase_hint_after_failure_*.png` | `GET INTO THE ALCOVE` after a checkpoint respawn (TRY 2) | `--act1-pilot skip`, tick 1010 |
| `gif_f10_first_run_640x360.gif` | ACT1-F10 in the real window: ticks 700 to 936 (the tell, the commit, the pass, the impact) every 4th tick at 70 ms, so about real time | `--capture-sequence 700:4:60`, frames joined with PIL |
| `gif_f13_lag448_schematic.gif` | ACT1-F13 at lag 448 as a **schematic** drawn from the fixture's per-tick trace (it is not a window capture: the binary has no two-pad script), ticks 500 to 1200 every 4th tick | `cargo test --lib -- --ignored print_the_f13_trace --nocapture` and a PIL drawing |
| `f13_commit_table.txt` | The F13 commit table (commit tick, commit front, margin per hero, separation at the arm, per lag), the rescue counts by spot of every chase fixture (F10, F10b, F15 included) and the 96-run sweep line | `cargo test --lib -- --ignored print_the_chase_tables --nocapture` |
| `perf.txt` | Frame time of the chase against room 3, three runs per view | see below |
| `capture_lines.txt` | One `ACT1_STILL` line per still and size: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`); 21 of 21 (ACT1-A) and 30 of 30 (ACT1-B) | the commands below |
| `fixtures.txt` | The eleven ACT1-A fixtures (F01 to F08: ticks, exit, defeats, grants, door openings, casts, final hash) and the twelve chase fixtures (F10 to F16: the event ticks, wheel hits, rescues by spot, commit front, final hash) | `cargo test --lib -- --ignored print_the_case_outcomes --nocapture` and `print_the_chase_tables` |
| `run_walk.txt` | The entry line of each of the **fifteen** rooms of the floor run, in order, with the next room. These are per-room launches on the static pins (one headless smoke per manifest row, no carry hand-off; the Juicer has no entry line of its own), not one carried `--f01-run` | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `run_chain.txt` | The carried chain through all fifteen rooms (ledger carried in and handed on). The Act I rooms and the corral carry the empty ledger: `unchanged=false` on line 1 only means the first room (now the chase) has no carried ledger and hands on the empty one; the guild's biscuit never enters the ledger (read-through) | `cargo test --lib -- --ignored print_the_carried_run_chain --nocapture`; asserted by `act1_carry_is_empty_and_accepted`, `the_chase_exit_relaunches_into_the_shore` and `the_guild_exit_relaunches_into_the_corral` |
| `playtrace_verify.txt` | The PLAYTRACE recording of each Act I room verifies strict at 360 and 400, under Story and All (12 lines; the chase's is the F10 script, 1377 ticks) | `cargo test --lib -- f01_act1_chase_recording_verifies f01_act1_shore_recording_verifies f01_act1_guild_recording_verifies --nocapture` |
| `checks.txt` | The local check list on the final code head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes, three sizes; the chase uses `--f01-room act1_chase --act1-pilot hide|skip|watch`; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`, `640x400` is `--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room act1_shore|act1_guild --act1-pilot strike|donut --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--act1-pilot` is the evidence pilot of the Act I rooms (`act1_fixtures::pilot`, a pure function of the finished state). `strike` plays in the shore (Carl to the strike column, three jabs, hold still while the block falls, then fight on and walk out), `donut` in the guild (swap to Donut on the first tick, walk right through the zone and the door), `hide` in the chase (Carl runs to the alcove, waits for the wedge and walks out: the F10 script), `skip` (Carl waits at the closed barricade, then plays `hide` after the respawn) and `watch` (Carl holds just past the trigger so the capture sees the emergence) in the chase, `walk` in any. It is for captures and headless runs only; a live run is played by a human, and a floor run refuses it.

## Readings

- **Shore** (`act1_shore_solo_carl`): companion off, Carl defeats both goblins in melee and walks out, exit on tick 274, no `UnlockGranted`, no Donut cast. `act1_shore_2p` and `act1_shore_solo_companion` (default `cf02_free`) end the same way, Donut never casts (CF02-Q17).
- **Block kill** (`act1_shore_block_kill`): the third jab at local 348 breaks the beam on tick 163; the block lands and defeats `gob_a` on tick 199 (cause `ShoreBlock { player: 0 }`, credited to Carl); `gob_b` falls to melee on tick 263; exit on tick 358.
- **Retry** (`act1_shore_retry_from_post_chase`): after a goblin was hurt and walked off its column, a checkpoint respawn puts the party at local 56 and both goblins are back on their columns 528 and 568 at full health; no machine anywhere (no slot enemy).
- **Guild** (`act1_guild_solo_donut`, `_solo_carl`, `_2p`, `_companion_crossing`): exactly one `UnlockGranted { hero: 1 }` when Donut's pivot enters 336..376 (ticks 77, 89, 76 and 123), the door-open event in the same tick, the door solid gone in the same tick's env step (heroes collided with it earlier in that tick, so they first pass it the next tick), exit on ticks 137, 149, 136 and 142. The panel fact is `(Donut, primary_ranged)` (a read-through: the ledger holds nothing).
- **Retry and restart** (`act1_guild_retry_keeps_the_biscuit`): a checkpoint respawn keeps the biscuit and the open door, no second grant; a fixture restart closes the door and takes the biscuit back; the second crossing grants once and opens the door once (events on tick 77 of attempt 1 and tick 78 of attempt 2).
- **All loadout** (`act1_guild_all_loadout_is_open`): the biscuit is held at tick 0, the door is open at tick 0, no grant and no door event.
- **The door is a wall** (`the_closed_door_blocks_carl_until_the_tick_after_the_unlock`): with Donut idle (2P) Carl stops at x 516 for as long as the door stands, is still held at the end of the unlock tick and is through two ticks later. **Never sealed in** (`the_trailing_bot_walks_the_zone_so_the_room_never_softlocks`): with the solo follow bot the trailing bot walks the zone before Carl reaches the door.
- **Panels**: `ROOM 1/15 CLEARED: THE FIRST CHASE`, `ROOM 2/15 CLEARED: THE ROTTEN SHORE`, `ROOM 3/15 CLEARED: THE GUILD DOOR` (with `primary_ranged (Donut)`), then the corral; no broadcast line before the corral.

### The chase (ACT1-B)

- **First run** (`act1_chase_solo_first_run`, `_donut`): the scripted hero waits at the pocket's right end, the follow bot trails 48 px behind and stands inside before the tell (zero `AiRescue`). Events in order: rumble (tick 341), emerged (431), timber a (502), `ChargeArmed` (589), timber b (605), the heroes in the pocket (653, 670), the tell (747), committed (783), `BarrierImpact` (873), `ChargeEnded` and the one `BrakeStarted` at `stop_x` (1008), `Wedged` (1123), the exit (1377). The commit front is **1857.6** (the window is 1841..1932), the gap 220, the margin of both heroes 66 ticks against the 20-tick reserve. Zero wheel contacts, no respawn. The impact falls exactly 90 ticks after the commit (the charge's last allowed tick).
- **Hull** (`act1_chase_hull_cannot_be_hurt`): four `Clang`s from Carl's jabs, hp constant 255, no stun, no defeat.
- **2P** (`act1_chase_2p_together`, `act1_chase_2p_lag_{0,96,224,448}`): zero wheel contacts, the commit front 1858.4 in every case, the impact inside the charge; the separation at the leader's arm is 6, 99, 230 and 444 px (the camera leash stops the leader at 447, so 448 is held at 440: ACT1-IMP-13). Rule 4 is the last rule to hold at the cruise, so the lag only moves the commit tick (733, 764, 805, 987).
- **No charge** (`act1_chase_no_charge_path`): P2 shadows the front 115 px ahead, rule 2 never holds, there is no tell and no charge; the cruise passes the pocket, breaks the barricade (tick 954), brakes and wedges; zero wheel contacts, both heroes exit.
- **Skip the pocket** (`act1_chase_skip_pocket`): Carl waits at 2300; no tell; the wheel defeats the bot first (it left the pocket to follow him, 21 `WheelHit` events while it lies under the wheel) and Carl at tick 903, the machine breaks the barricade at tick 913 during the defeat delay (ACT1-IMP-11), the respawn rebuilds it (dormant, `barrier_hit()` false, the barricade standing again, the hint live) and a clean run wedges and exits.
- **Bots** (`act1_chase_solo_bot_{carl,donut}_active`, default companion): zero wheel contacts; with Carl active the companion was rescued twice into the pocket (`off_screen` at tick 560, `charge` at the tell, tick 831); with Donut active none. Across the 96-run sweep (either hero active, the follow bot or the default companion, four start delays, six stalls) there are zero wheel contacts and no respawn; 56 bot rescues in all, none to the lane or the tunnel spot.
- **Frame time** (`perf.txt`, debug build, xvfb, loadavg about 2 to 5): the chase's median interval is 16.60 to 16.75 ms and p95 18.95 to 21.21 ms over 1380 frames; room 3's `dozer:run` is 16.54 to 16.68 ms and p95 18.61 to 20.09 ms over 345 frames. The interval is X-copy paced (a hypothesis, see `scripts/room_perf.sh`), so it does not separate the rooms; the simulation tick is 17 to 26 us in the chase (the pilot plays Carl only, the follow bot) against 70 to 80 us in room 3's pilot run (the `cf02_free` partner and its nav feed). No frame-time regression is visible. This is not a real-display or handheld measurement.

## Not run

- A real two-pad run: **NOT RUN**.
- A human first-run time and human recognition of the guide and the door: **NOT RUN**.
- A real-window two-pad run of the 2P cases (the F13 GIF is a schematic) and human recognition of the alcove: **NOT RUN**.
- Audio (ACT1-Q2), the end-on wedge art (ACT1-Q3) and the ENV-02 dressing (ACT1-Q7): later work orders.

## Reproduce the check list

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
scripts/ci_floor01_combat.sh headless      # and: render (under xvfb-run -s '-screen 0 1920x1080x24')
scripts/check_f01_owner_commands.sh
```
