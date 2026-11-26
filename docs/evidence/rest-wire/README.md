# REST-WIRE evidence: the refuge (REST-A, PR 1)

Room 1 of the floor run, `rest_refuge` ("The Wrong Restaurant"), launched with `--scene f01_late --f01-room rest_refuge` and walked by `--f01-run` right after the corral. It is a **labelled playable greybox** (flat decor rectangles on the existing kit; the AREA-FASTFOOD-01 art is a separate later work order). Design: [floor-01-quiet-setpieces-and-transitions.md](../../01-design/floor-01-quiet-setpieces-and-transitions.md) section G. Owner decision of 2026-11-25 ("go with all"): everything on by default, reachable in story order. Game feel is the owner's call: nothing here is accepted.

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_threshold_*.png` | The refuge threshold (x 0..224, the protected spawn) | window, `--rest-pilot donut`, tick 20 |
| `still_light_feet_notice_*.png` | Donut past the reward box: the `LIGHT FEET` grant notice (grant on tick 326) | `--rest-pilot donut`, tick 340 |
| `still_shelf_door_open_*.png` | Donut standing on the 128 px shelf, the staff door gone (opened on tick 370) | `--rest-pilot donut`, tick 376 |
| `still_swap_hint_*.png` | Solo Carl waiting at the closed staff door: the `SWAP TO DONUT` notice | `--rest-pilot carl_door`, tick 560 |
| `before_after_strip.png` (from `still_strip_*`) | Top row: a base jump from before the box peaks 13 px short of the shelf top. Bottom row: the boosted jump lands on the shelf and the door opens | `--rest-pilot base_jump` ticks 308, 324, 348; `--rest-pilot donut` ticks 336, 360, 374; `montage` |
| `capture_lines.txt` | One `REST_STILL` line per still and size: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`) | the commands below |
| `run_walk.txt` | The entry line of each of the **ten** rooms of the floor run, in order, with the next room | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `audience_run.txt` | AUDIENCE-RUN viewer counts at the refuge's entry and exit | `cargo test --lib -- the_refuge_runs_the_audience --nocapture` |
| `playtrace_verify.txt` | The refuge's PLAYTRACE recording verifies strict at 360 and 400, under Story and All | `cargo test --lib -- f01_rest_refuge_recording_verifies --nocapture` |
| `fixtures.txt` | The seven scripted fixtures: ticks, grants, door openings, damage, final hash | `cargo test --lib -- --ignored print_the_case_outcomes --nocapture` |
| `physics_exact.txt` | K8 on the launched kit: every accepted edge of every class lands with the real integrator; the exit walk | `cargo test --lib -- rest_refuge_shelf_is_boosted_only --nocapture` |
| `checks.txt` | The local check list on the final head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes, three sizes; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`, `640x400` is `--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room rest_refuge --rest-pilot donut|carl_door|base_jump --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--rest-pilot` is the evidence pilot of the refuge (`rest_fixtures::pilot`, a pure function of the finished state; its first press is the solo swap to Donut). The pilot is for captures and headless runs only; a live run is played by a human.

## Readings

- **The crossing** (fixture `rest_refuge_solo_donut`, the same run as the `donut` pilot): the box grants `jump_reach_boost` once on tick 326 (Donut's pivot at x 1201), the door-open event is on tick 370 (the landing tick, feet on the shelf top), the exit is reached on tick 414, no damage. One receipt.
- **Solo swap** (`rest_refuge_solo_carl_swap`): Carl walks to the door with Donut following (she crosses the box on the way, tick 369), waits, swaps; Donut opens the door on tick 597. **The default `cf02_free` companion did not open the door by herself** (`rest_refuge_solo_companion_crossing`: Carl waits at the closed door and the human swaps, as the hint asks); the door is latched either way.
- **Base Donut** cannot open it (`rest_refuge_base_donut_cannot_open`, no story flags: no grant, no door, no exit in 1500 ticks). A flagless launch is sealed by design (`boost=none` in the entry line).
- **Retry** (`rest_refuge_retry_keeps_the_grant`): a checkpoint respawn keeps the boost and the open door; a fixture restart drops both and re-arms the box; the second crossing grants once and opens the door once.
- **AUDIENCE-RUN (REST-AD14, REST-Q10 open):** the refuge is the first live room and starts at the baseline (250); a quiet crossing has no kill, so no award; the 414 ticks are inside the 480-step decay delay, so the viewers leave at the baseline. Nothing is frozen; whether decay should freeze in quiet rooms is the owner's call.
- **The hint** appears 90 ticks after Carl stood within 96 px of the door. The notice queue holds it behind a showing grant notice (`LIGHT FEET`), so it can start a little later than the 90th tick when Carl arrives while the notice is still on screen.

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

A real two-pad run, a human session, the AREA-FASTFOOD-01 art, and the camp and aftermath rooms (REST-WIRE PR 2).
