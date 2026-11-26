# ACT1-A evidence: the shore and the guild at the start of the floor run

The first two rooms of the fourteen-room floor run: the shore, `act1_shore` ("The Rotten Shore"), and the guild, `act1_guild` ("The Guild Door", GUILD-BEAT-01, Donut's biscuit). Both launch with `--scene f01_late --f01-room ID` and are walked by `--f01-run` before the corral. They are **labelled playable greybox** (flat decor rectangles on the existing kit; the ENV-02 dressing is ACT1-Q7, audio is ACT1-Q2). Design: [floor-01-act-01-integration.md](../../01-design/floor-01-act-01-integration.md) (section 17 holds the ACT1-IMP defaults). The chase room, `act1_chase`, is ACT1-B and **not** in this PR. Owner decision of 2026-11-25 ("Go with all because I could not test it until next week" and "Ensure that everything ends on main for testing all together next week"): everything on by default, reachable in story order. Game feel is the owner's call: nothing here is accepted.

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_shore_spawn_*.png` | The shore at spawn: the wedged machine decor (flat dark rectangle) at the left edge | window, `--f01-room act1_shore --act1-pilot strike`, tick 20 |
| `still_shore_strike_beam_*.png` | Carl at the strike column (local 348), the rotten beam after the first hit, the first goblin walking in | same, tick 100 |
| `still_shore_block_falling_*.png` | The beam broken, the block falling over the goblin that followed Carl (footprint local 392..472) | same, tick 196 |
| `still_shore_block_kill_*.png` | The block landed; the goblin under it is defeated (credited to the block) | same, tick 212 |
| `still_guild_door_closed_*.png` | Donut before the zone: the prompt reads `SWAT`, the `guild_door` stands (brown column), the guide decor | window, `--f01-room act1_guild --act1-pilot donut`, tick 60 |
| `still_guild_missile_notice_*.png` | Donut in the zone: the `MAGIC MISSILE` notice and the prompt changed to `MISSILE` (the biscuit, tick 77) | same, tick 92 |
| `still_guild_door_open_*.png` | The door gone after the unlock (open event in tick 77, not colliding from tick 78) | same, tick 112 |
| `capture_lines.txt` | One `ACT1_STILL` line per still and size: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`); 21 of 21 | the commands below |
| `fixtures.txt` | The eleven scripted fixtures (ACT1-F01 to F08): ticks, exit, defeats, grants, door openings, casts, final hash | `cargo test --lib -- --ignored print_the_case_outcomes --nocapture` |
| `run_walk.txt` | The entry line of each of the **fourteen** rooms of the floor run, in order, with the next room. These are per-room launches on the static pins (one headless smoke per manifest row, no carry hand-off; the Juicer has no entry line of its own), not one carried `--f01-run` | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `run_chain.txt` | The carried chain through all fourteen rooms (ledger carried in and handed on). The Act I rooms and the corral carry the empty ledger: `unchanged=false` on line 1 only means the first room has no carried ledger and hands on the empty one; the guild's biscuit never enters the ledger (read-through) | `cargo test --lib -- --ignored print_the_carried_run_chain --nocapture`; asserted by `act1_carry_is_empty_and_accepted` and `the_guild_exit_relaunches_into_the_corral` |
| `playtrace_verify.txt` | The PLAYTRACE recording of each Act I room verifies strict at 360 and 400, under Story and All (8 lines) | `cargo test --lib -- f01_act1_shore_recording_verifies f01_act1_guild_recording_verifies --nocapture` |
| `checks.txt` | The local check list on the final code head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes, three sizes; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`, `640x400` is `--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room act1_shore|act1_guild --act1-pilot strike|donut --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--act1-pilot` is the evidence pilot of the Act I rooms (`act1_fixtures::pilot`, a pure function of the finished state). `strike` plays in the shore (Carl to the strike column, three jabs, hold still while the block falls, then fight on and walk out), `donut` in the guild (swap to Donut on the first tick, walk right through the zone and the door), `walk` in either. It is for captures and headless runs only; a live run is played by a human, and a floor run refuses it.

## Readings

- **Shore** (`act1_shore_solo_carl`): companion off, Carl defeats both goblins in melee and walks out, exit on tick 274, no `UnlockGranted`, no Donut cast. `act1_shore_2p` and `act1_shore_solo_companion` (default `cf02_free`) end the same way, Donut never casts (CF02-Q17).
- **Block kill** (`act1_shore_block_kill`): the third jab at local 348 breaks the beam on tick 163; the block lands and defeats `gob_a` on tick 199 (cause `ShoreBlock { player: 0 }`, credited to Carl); `gob_b` falls to melee on tick 263; exit on tick 358.
- **Retry** (`act1_shore_retry_from_post_chase`): a checkpoint respawn puts the party at local 56; both goblins are alive again at their columns; no machine anywhere (no slot enemy).
- **Guild** (`act1_guild_solo_donut`, `_solo_carl`, `_2p`, `_companion_crossing`): exactly one `UnlockGranted { hero: 1 }` when Donut's pivot enters 336..376 (ticks 77, 89, 76 and 123), the door-open event in the same tick, the door solid gone in the same tick's env step (heroes collided with it earlier in that tick, so they first pass it the next tick), exit on ticks 137, 149, 136 and 142. The panel fact is `(Donut, primary_ranged)` (a read-through: the ledger holds nothing).
- **Retry and restart** (`act1_guild_retry_keeps_the_biscuit`): a checkpoint respawn keeps the biscuit and the open door, no second grant; a fixture restart closes the door and takes the biscuit back; the second crossing grants once and opens the door once (events on tick 77 of attempt 1 and tick 78 of attempt 2).
- **All loadout** (`act1_guild_all_loadout_is_open`): the biscuit is held at tick 0, the door is open at tick 0, no grant and no door event.
- **Never sealed in** (`carl_alone_cannot_pass_the_closed_door`): Carl stops at the closed door; the trailing companion walks the zone on her own, so the room never softlocks.
- **Panels**: `ROOM 1/14 CLEARED: THE ROTTEN SHORE`, `ROOM 2/14 CLEARED: THE GUILD DOOR` (with `primary_ranged (Donut)`), then the corral; no broadcast line before the corral.

## Not run

- A real two-pad run: **NOT RUN**.
- A human first-run time and human recognition of the guide and the door: **NOT RUN**.
- The chase (`act1_chase`), the opening profile, audio and the end-on wedge art are ACT1-B / later.

## Reproduce the check list

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
scripts/ci_floor01_combat.sh headless      # and: render (under xvfb-run -s '-screen 0 1920x1080x24')
scripts/check_f01_owner_commands.sh
```
