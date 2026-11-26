# REST-WIRE evidence: the refuge, the camp and the aftermath (REST-A in PR 1, REST-B and REST-C in PR 2)

The three quiet rooms of the twelve-room floor run. Room 2 of the run, `rest_refuge` ("The Wrong Restaurant"), launched with `--scene f01_late --f01-room rest_refuge` and walked by `--f01-run` right after the corral. It is a **labelled playable greybox** (flat decor rectangles on the existing kit; the AREA-FASTFOOD-01 art is a separate later work order). Design: [floor-01-quiet-setpieces-and-transitions.md](../../01-design/floor-01-quiet-setpieces-and-transitions.md) section G. The camp (`rest_camp`, "The Survivors' Camp", after `ring_split`) and the aftermath (`rest_aftermath`, "After the Juicer", after the Juicer) are PR 2 and the same kind of labelled playable greybox (AREA-CAMP-01 and the gym and arena dressing are separate later orders). Owner decision of 2026-11-25 ("go with all"): everything on by default, reachable in story order. Game feel is the owner's call: nothing here is accepted.

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_threshold_*.png` | The refuge threshold (x 0..224, the protected spawn) | window, `--rest-pilot donut`, tick 20 |
| `still_light_feet_notice_*.png` | Donut past the reward box: the `LIGHT FEET` grant notice (grant on tick 326) | `--rest-pilot donut`, tick 340 |
| `still_shelf_door_open_*.png` | Donut standing on the 128 px shelf, the staff door gone (opened on tick 370) | `--rest-pilot donut`, tick 376 |
| `still_swap_hint_*.png` | Solo Carl waiting at the closed staff door: the `SWAP TO DONUT` notice | `--rest-pilot carl_door`, tick 560 |
| `before_after_strip.png` (from `still_strip_*`) | Top row: a base jump from before the box peaks 13 px short of the shelf top. Bottom row: the boosted jump lands on the shelf and the door opens | `--rest-pilot base_jump` ticks 308, 324, 348; `--rest-pilot donut` ticks 336, 360, 374; `montage` |
| `still_camp_*.png` | PR 2: the camp entrance (the `ring_marker_a` stand-in pole at x 220 and the sentry at x 512), the central light over the checkpoint (crate stack, light, repair figure), the chalk marks over the last 128 px | window, `--f01-room rest_camp --rest-pilot walk`, ticks 70, 200, 390 |
| `still_aftermath_*.png` | PR 2: the wrecked-gym egress and the heavy bag, Donut at the blink lane's near mark and on the far side after the blink (tick 201 press, tick 207 arrival), the circular grooves at the exit | window, `--f01-room rest_aftermath --rest-pilot blink`, ticks 60, 196, 214, 300 |
| `capture_lines_bc.txt` | PR 2: one `REST_STILL` line per camp and aftermath still and size, same checks as below | the commands below |
| `capture_lines.txt` | One `REST_STILL` line per still and size: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`) | the commands below |
| `run_walk.txt` | The entry line of each of the **twelve** rooms of the floor run, in order, with the next room | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `audience_run.txt` | AUDIENCE-RUN viewer counts at the entry and exit of each quiet room (the refuge from the baseline; the camp and the aftermath resumed at a Ring-sized peak 1400/1900 and 600/800, the carry a run relaunch would hand on) | `cargo test --lib -- the_refuge_runs_the_audience the_camp_and_the_aftermath_carry --nocapture` |
| `playtrace_verify.txt` | The PLAYTRACE recording of each quiet room verifies strict at 360 and 400, under Story and All (12 lines) | `cargo test --lib -- f01_rest_refuge_recording_verifies f01_rest_camp_recording_verifies f01_rest_aftermath_recording_verifies --nocapture` |
| `fixtures.txt` | The twelve scripted fixtures (seven refuge, two camp, three aftermath): ticks, grants, door openings, damage, final hash | `cargo test --lib -- --ignored print_the_case_outcomes --nocapture` |
| `physics_exact.txt` | K8 on the launched kit: every accepted edge of every class lands with the real integrator; the exit walk | `cargo test --lib -- rest_refuge_shelf_is_boosted_only --nocapture` |
| `checks.txt` | The local check list on the final head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes, three sizes; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`, `640x400` is `--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room rest_refuge --rest-pilot donut|carl_door|base_jump --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--rest-pilot` is the evidence pilot of the quiet rooms (`rest_fixtures::pilot`, a pure function of the finished state; the Donut pilots start with the solo swap). `donut`, `carl_door` and `base_jump` play in the refuge, `walk` (Carl walks to the exit) in any quiet room, `blink` (Donut to the lane's near mark, one blink, on to the exit) in the aftermath. The pilot is for captures and headless runs only; a live run is played by a human. PR 2 stills use `--f01-room rest_camp|rest_aftermath` with `--rest-pilot walk|blink`, `--capture-sequence TICK:1:1`.

## Readings

- **The crossing** (fixture `rest_refuge_solo_donut`, the same run as the `donut` pilot): the box grants `jump_reach_boost` once on tick 326 (Donut's pivot at x 1201), the door-open event is on tick 370 (the landing tick, feet on the shelf top), the exit is reached on tick 414, no damage. One receipt.
- **Solo swap** (`rest_refuge_solo_carl_swap`): Carl walks to the door with Donut following (she crosses the box on the way, tick 369), waits, swaps; Donut opens the door on tick 597. **The default `cf02_free` companion did not open the door by herself** (`rest_refuge_solo_companion_crossing`: Carl waits at the closed door and the human swaps, as the hint asks); the door is latched either way.
- **Base Donut** cannot open it (`rest_refuge_base_donut_cannot_open`, no story flags: no grant, no door, no exit in 1500 ticks). A flagless launch is sealed by design (`boost=none` in the entry line).
- **Retry** (`rest_refuge_retry_keeps_the_grant`): a checkpoint respawn keeps the boost and the open door; a fixture restart drops both and re-arms the box; the second crossing grants once and opens the door once.
- **AUDIENCE-RUN (REST-AD14, REST-Q10 open):** the refuge is the first live room and starts at the baseline (250); a quiet crossing has no kill, so no award; the 414 ticks are inside the 480-step decay delay, so the viewers leave at the baseline. Nothing is frozen; whether decay should freeze in quiet rooms is the owner's call.
- **The hint** appears 90 ticks after Carl stood within 96 px of the door. The notice queue holds it behind a showing grant notice (`LIGHT FEET`), so it can start a little later than the 90th tick when Carl arrives while the notice is still on screen.

- **The camp** (`rest_camp_crossing_solo`, `_2p`): no grant, no door, no damage, the exit on tick 432 solo and 396 in 2P; the ledger at the exit equals the post-Hoarder pin's (dynamite and Light on Your Feet held, no receipt). The crate stack is decor, not a platform (REST-IMP-8).
- **The aftermath** (`rest_aftermath_no_duplicate_unlock`): entered with a post-Juicer ledger of four capabilities and four receipts, it grants nothing and the exit snapshot equals the carry (exit on tick 349). `rest_aftermath_crossing_base_moves` crosses it with no story flag (tick 349). `rest_aftermath_blink_lane`: Donut stops at x 767, presses the blink on tick 201, arrives at x 879 on tick 207 (112 px, no rejection, fizzle or cancel), mana spent once (8 to 6), the exit on tick 317. The heavy bag is an inert static (REST-IMP-9).
- **The twelve-room run** (`run_walk.txt`): corral, rest_refuge, hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery; the manifest-driven run needed no run code, only the manifest rows. The panels read ROOM n/12.
- **AUDIENCE-RUN (REST-AD14, REST-Q10 open):** the camp and the aftermath are live (the rule is positional). Both crossings (432 and 349 ticks) end inside the 480-step decay delay, so the counts leave exactly as they entered and the peaks are kept; a slower player would see the counts decay unchanged (nothing is frozen).

## Reproduce the check list

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
scripts/ci_floor01_combat.sh headless      # and: render (under xvfb-run -s '-screen 0 1920x1080x24')
scripts/check_f01_owner_commands.sh
```

## NOT RUN

A real two-pad run, a human session, the AREA-FASTFOOD-01 and AREA-CAMP-01 art and the gym and arena dressing.
