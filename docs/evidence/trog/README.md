# TROG-A evidence: the three troglodytes and `trog_run` in the floor run

`trog_run` ("The Troglodyte Run", #191 T0..T3) is listed after `rest_camp` and directly before the Juicer, so the floor run is sixteen rooms. One Pygmy, one Basher and one Virtuoso run on ENEMY-SLOT. They are **labelled playable greybox**: the enemies draw the committed TROG-01/02/03 atlas frames (`prototype_fixture_unapproved`, **art review pending**; frame map TROG-AD39..AD41) with a translucent stand-in telegraph of the committed hitbox, and the room is flat decor rectangles on the existing kit. The labelled hurtbox rectangle is the fallback where an atlas is missing and, faint, behind `--debug-combat`. Design: [the troglodyte integration plan](../../01-design/floor-01-troglodyte-integration.md) (adopted defaults TROG-AD1..AD41, implementation notes 15.1); work order: [the troglodyte work order](../../07-agents/floor-01-troglodyte-workorder.md). Owner decisions of 2026-11-24 and 2026-11-25: everything on by default, reachable in story order, large unfinished items land as a labelled greybox. Game feel is the owner's call: nothing here is accepted. The Test of Strength, the bench and the gate are TROG-B (not in this evidence).

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_pygmy_dormant_*.png` | T1: the Pygmy asleep at 640 (the idle frame), Carl walking in | window, `--f01-room trog_run --trog-pilot carl --companion off`, tick 80 |
| `still_pygmy_tell_*.png` | The Pygmy's 24-tick coil (tell tick 11 of 24): the translucent box is the committed hitbox | same, tick 125 |
| `still_pygmy_leap_*.png` | Mid-leap: the committed arc, the body lifted on the integer parabola | same, tick 144 |
| `still_pygmy_land_*.png` | The landing: Recover, the low hurtbox the stomp meets | same, tick 152 |
| `still_basher_tell_*.png` | T2: the Basher's 36-tick overhead tell with its smash rectangle committed | same, tick 520 |
| `still_basher_smash_*.png` | The impact window (ticks 39..46 of the attack) | same, tick 543 |
| `still_basher_recover_*.png` | The 20-tick recovery the pilot punishes | same, tick 556 |
| `still_virtuoso_tell_*.png` | T3: the Virtuoso's 30-tick feint with its committed band (36..46 px up) | same, tick 870 |
| `still_virtuoso_lash_*.png` | The lash and the strike window | same, tick 893 |
| `still_exit_open_*.png` | All three defeated: the exit gate open | same, tick 960 |
| `gif_pilot_carl_vs_pygmy_640x360.gif`, `gif_pilot_carl_vs_basher_640x360.gif` | The real window at 1x, every 4th tick at 70 ms (about real time): the pilot's Carl against the Pygmy (ticks 100..216) and the Basher (ticks 500..640). They show the **pilot** (`--trog-pilot carl`), not the scripted fixtures F01 and F03 | `--capture-sequence 100:4:30` and `500:4:36`, frames joined with PIL |
| `capture_lines.txt` | One `TROG_STILL` line per still and size: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`); 30 of 30 | the commands below |
| `fixtures.txt` | The scene fixtures F01..F04 and F07 (ticks, exit, defeats, hurts, tells, whiffs, interrupts, final hash) and F08's facts | `cargo test --lib -- --ignored print_the_fixture_lines --nocapture` |
| `run_walk.txt` | The entry line of each of the **sixteen** rooms of the floor run, in order (one headless smoke per manifest row, no carry hand-off): `rest_camp`, `trog_run`, `juicer` | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `run_chain.txt` | The carried chain through all sixteen rooms: `trog_run` carries the camp's ledger in and hands it on unchanged (`held:2,receipts:1`); the Juicer's exit is the next change | `cargo test --lib -- --ignored print_the_carried_run_chain --nocapture`; asserted by `the_camp_and_aftermath_carry_their_ledger_through` |
| `playtrace_verify.txt` | The PLAYTRACE recording of `trog_run` verifies strict at 360 and 400, under Story and All (4 lines) | `cargo test --lib -- f01_trog_run_recording_verifies --nocapture` |
| `checks.txt` | The local check list on the final code head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes, three sizes; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`, `640x400` is `--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room trog_run --trog-pilot carl --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--trog-pilot carl|donut` is the evidence pilot of the troglodyte room (`trog_fixtures::pilot`, a pure function of the finished state). `carl` fights the troglodytes in order with the brawler policy and walks to the exit; `donut` swaps to Donut on the first tick, who casts at them from the floor. It is for captures and headless runs only; a live run is played by a human, and a floor run refuses it.

## Readings

- **Commit** (`trog_commit_at_tell_start_*`, `an_attack_lands_one_blow_per_hero_per_instance`, `a_blow_never_lands_before_the_active_window`): facing, the Pygmy's landing x, the Basher's smash rectangle and the Virtuoso's line are fixed on the tell's first tick. A hero who leaves is not hit. One blow per hero per attack instance, and never before the active window.
- **Pygmy** (F01, `trog_pygmy_dodge_and_punish`): Carl placed at L - 48 facing the landing takes no blow (a `Whiff`), stomps on the first land tick (the stomp hits on its tick 9 of the 14-tick land, 1 damage, no reaction), then jabs in Rest and defeats it: one cycle, 0 damage, 115 ticks. **F02** (the positive control): Donut planted at 432 casts and defeats it in 59 ticks with no tell and no hurt.
- **Basher** (F03): one `Interrupted` inside the first 18 tell ticks, then Carl backs out of every later smash (`Whiff`) and jabs the recovery: defeated in two tells, 0 damage. **F04**: Donut's bolts defeat it from 200 px, none interrupts a tell.
- **Virtuoso** (F05, F06): the tongue hits grounded Carl, passes over a grounded Donut (0..24), hits Donut with her feet at 13..45, hits both on the 32 px step and neither on the 64 px platform. With both heroes present every tell names Carl while Donut is on the floor; a lone grounded Donut is never told at and defeats it from the floor (TROG-Q13). A raised Donut dodges the lines by a hop timed to the feint and bolts the retract (bolts at 48..54 meet the 0..52 retract hurtbox by 4 px, re-review R2).
- **Run** (F07): solo Carl (companion parked) clears the room in 997 ticks, solo Donut in 763, 2P in 777, solo Carl with the default `cf02_free` companion in 997, with no hero defeat and no hurt in any scripted run. Every wake is a proximity wake with the cause logged. The companion's goal layer evaluates the troglodytes through the slot feed (`the_companion_sees_the_troglodytes_through_the_slot_feed`); the scripted Carl clears each enemy before she casts.
- **Retry** (F08): a party defeat in T2, then the checkpoint respawn: the defeated Pygmy stays down, the living Basher is back at 1376, whole and asleep; a fixture restart rebuilds all three.
- **Reach** (`a_hero_out_of_the_leash_reach_is_not_a_candidate`): a hero on the T1 refuge (pivot 404 or less) is out of the Pygmy's leash reach and is not its target (TROG-AD31). A blow's knock is about 60 px, so a hero knocked out of reach stops drawing the Pygmy and the Basher.
- **Validator** (TROG-T05): V0..V4 each reject a minimal broken sidecar; the shipped `trog_run` sidecar passes them all.
- **Hashes**: no committed golden moved (full suite and `scripts/ci_floor01_combat.sh headless` byte-identical); the moved pins are listed in the PR.

## Not run / not working

- A real two-pad run and a human first-run time: **NOT RUN**. The mixed fight's feel (TROG acceptance question 9) is the owner's.
- The T3 platform with the tongue passing under it, as a capture: **NOT RUN**. A raised Donut is a Virtuoso candidate only while airborne at feet 13..45, and the 32 px step lies 36 px from the platform, so no pilot can stand a hero on the platform while the other draws the tongue; the geometry is asserted against the real room file (`the_t3_platform_stands_over_the_tongue_in_the_real_room`) and by the F05 matrix.
- A 2P window capture and `trog_eval`/the gate: TROG-B.
- The atlases' own independent art review, audio and the AREA-TROG-01 dressing: later work orders. Base Donut cannot reach the 64 px platform (TROG-AD35).

## Reproduce the check list

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
scripts/ci_floor01_combat.sh headless
xvfb-run -a -s '-screen 0 1920x1080x24' scripts/ci_floor01_combat.sh render
scripts/check_f01_owner_commands.sh
```
