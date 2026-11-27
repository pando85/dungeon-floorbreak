# TROG-B evidence: the Test of Strength, `trog_eval` and the hand-off into the Juicer

`trog_strength` ("The Test of Strength", #191 T4) is listed after `trog_run` and directly before the Juicer, so the floor run is **seventeen rooms** and walks `rest_camp`, `trog_run`, `trog_strength`, `juicer`. One Virtuoso and one Basher run on ENEMY-SLOT with the TROG-A machine unchanged. It is a **labelled playable greybox** (the enemies draw the committed TROG-01/02/03 atlas frames, `prototype_fixture_unapproved`, **art review pending**; the room is flat decor rectangles on the existing kit). Design: [the troglodyte integration plan](../../01-design/floor-01-troglodyte-integration.md) (adopted defaults TROG-AD42..AD45, implementation notes 15.2); work order: [the troglodyte work order](../../07-agents/floor-01-troglodyte-workorder.md). Game feel is the owner's call: nothing here is accepted.

## Read this first: the bench is an 80 px static and every gate row passes

1. **The 80 px `bench_stack` partition is refused by the sidecar validator** (`ObstacleTooTall`: a `solid_intact` prop leaves at most 48 px on the lane). That is the work order's first-action stop condition. The prop rules were **not** widened; the declared static fallback landed (plan 5.4, **TROG-AD43**): the bench is a static `solid` at 640..672, the Basher spawns at 800 (leash 704..896), and rule (b), the bench smash, `BenchSmash`, the partition shove, fixtures F10 to F12 and the bench debris presentation are cut. Test: `the_80_px_bench_stack_partition_is_refused_so_the_static_fallback_is_taken`.
2. **History: the first static bench was 48 px and failed G-1 and G-2** (**TROG-AD45**): from the far landing (x 720) a swept `stationary_jump_cast` at offsets 0 and 1 defeated both troglodytes in about 391 ticks with no damage (12 of 1,830 decisive runs); a bolt released at feet 32 to 54 flew over a 48 px bench and still met a hurtbox (tops 58 to 70).
3. **Now the bench is an 80 px static** (**TROG-AD46**, the plan author's decision): the validator gains `MAX_STATIC_SOLID_PX` 80 for a sidecar static `solid` only (Carl's class limit; the prop and remnant cap stays 48). A bolt below feet 64 stops on it and one from 64 up flies at 80 or above, over every hurtbox (70 or lower), so a stationary Donut can hit only the troglodyte on her own side. **The full sweep passes every row and exits 0** (`eval/report.txt`): G-1 1,830 gated runs none defeated both, G-2 0 of 610 groups, G-3, G-4 (both heroes cross the 80 px bench) and G-5; the grid (720 runs, bench tops at 80 included) finds no both-defeated run. TROG-Q14 (an 80 px partition, restoring the bench beat) stays open and non-blocking; TROG-Q15 is closed.

## Files

| File | What it shows | How it was made |
| --- | --- | --- |
| `still_strength_open_*.png` | The room's opening view: the shelf is ahead, the 80 px bench beyond it, the Virtuoso asleep at 496 | window, `--f01-room trog_strength --trog-pilot carl --companion off`, tick 30 |
| `still_virtuoso_tell_640x360.png` | The Virtuoso's feint with its committed band (tick 15 of the 30-tick tell) | same, tick 105 |
| `still_bench_climb_640x360.png` | Carl on top of the 80 px bench while the Basher stands asleep behind it (the `bench` and `shelf` are flat decor rectangles over the sidecar's statics, drawn while the room is plain, before its AREA-TROG-01 stage 2 dressing) | same, tick 270 |
| `still_basher_tell_*.png` | The Basher's overhead tell on the far side of the bench, the exit gate closed behind it | same, tick 360 |
| `still_basher_smash_640x360.png` | The impact window | same, tick 381 |
| `still_exit_open_640x360.png` | Both defeated: the exit gate open | same, tick 450 |
| `capture_lines.txt` | One `TROG_STILL` line per still: the window's `COMBAT_STATE` hash equals the headless hash of the same tick (`hash_match=yes`) and the two capture passes are byte-identical (`passes_byte_identical=yes`); 8 of 8. Honest note: one of four shoots of `exit_open` showed a 32 x 8 px difference between its two passes (presentation only; the window hash matched the headless hash every time); the committed lines are a shoot whose two passes were identical | the command below |
| `pilot_events.txt` | The carl pilot's act log (Virtuoso woken at 42, told at 90, defeated at 168; Basher woken at 238 (Carl crossed x 600), told at 340, defeated at 396; the exit at 476) and the ticks the stills use | `cargo test --lib -- --ignored print_the_strength_pilot_events --nocapture` |
| `eval/report.txt`, `eval/summary.json` | The `trog_eval` full sweep: 6,345 runs, the gate rows, the credit rows, the movement rows, the pockets and the grid, system failures. **every row passes, exit 0** | `trog_eval --out DIR` (about 23 s on 8 threads, debug build) |
| `corral_eval_identity.txt` | `corral_eval --out DIR` before and after this branch's `LlamaObs::rect` field: `summary.json`, `runs.csv` and `report.md` are byte-identical | `sha256sum` of the three files |
| `fixtures.txt` | The scene fixtures with their final hashes: the TROG-A lines (F01 to F04 and F07, F08) are **byte-identical** to `docs/evidence/trog/fixtures.txt`; new: F09, F13, F15 (2P and solo companion) and F14 (their hashes moved with the 80 px bench) | `cargo test --lib -- --ignored print_the_fixture_lines --nocapture` |
| `run_walk.txt` | The entry line of each of the **seventeen** rooms of the floor run, in order: `rest_camp`, `trog_run`, `trog_strength`, `juicer` | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-room ROOM` per manifest row |
| `run_chain.txt` | The carried chain through all seventeen rooms: `trog_run` and `trog_strength` carry the camp's ledger in and hand it on unchanged (`held:2,receipts:1`); the Juicer's exit is the next change (TROG-T04, TROG-T06) | `cargo test --lib -- --ignored print_the_carried_run_chain --nocapture`; asserted by `the_camp_and_aftermath_carry_their_ledger_through` |
| `playtrace_verify.txt` | The PLAYTRACE recordings of both troglodyte rooms verify strict at 360 and 400 under Story and All (8 lines) | `cargo test --lib -- f01_trog --nocapture` |
| `checks.txt` | The local check list on the final code head | see below |
| `SHA256SUMS` | Checksums of the files above | `sha256sum` |

Capture command (one still per run, two passes; `640x360` is `--scale 1`, `1920x1080` is `--scale 3`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S \
  --scene f01_late --f01-room trog_strength --trog-pilot carl --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`--trog-pilot carl` is the TROG-A evidence pilot (a pure function of the finished state); in the Test of Strength it fights the Virtuoso, hops the shelf and the bench (`with_hop`) and fights the Basher. It is for captures and headless runs only.

## TROG matrix (this PR)

| Row | Result | Where |
| --- | --- | --- |
| T04 `trog_rooms_listed_and_carry` | PASS | `f01_run` tests: the order, the carry through both rooms, the relaunch into `juicer` |
| T05 `trog_sidecars_validate` | PASS | `encounter::trog_v::tests`: the shipped sidecar passes every rule, V2 holds for the shelf and the bench, the 80 px partition is refused, a static solid is at most 80 px (81 refused) |
| T06 `trog_run_walk` | PASS | `run_walk.txt`, `run_chain.txt` |
| F09 `trog_strength_direct` | PASS | Carl climbs the 80 px bench and defeats the Basher on the far side; the Virtuoso is down (tick 168) before the Basher wakes (238); no blow lands |
| F10 bait, F11 bench by hand, F12 no break from the far side | **CUT** | the static fallback (TROG-AD43): nothing breaks |
| F13 `trog_strength_tell_gap` | PASS | both eligible at once: tells at least 24 ticks apart, no hero under two live hitboxes |
| F14 `trog_strength_retry_and_respawn` | PASS | the defeated Virtuoso stays down, the living Basher is back at 800 whole and asleep; a restart rebuilds both |
| F15 2P and solo companion | PASS | both clear it with no hero defeat; the bot's stuck time behind the bench is **not measured** (an 80 px static both heroes cross) |
| F16 `trog_strength_presentation_is_read_only` | PASS | F09's inputs with every presentation accessor read each tick give the same log and hash |
| F17 `trog_existing_hashes_unchanged` | PASS | the full suite: no committed golden, CF02, DP, DOZR, corral or Juicer value moved; TROG-A's fixture hashes are byte-identical |
| Gate G-1 hard invariant | PASS | 1,830 gated runs, none defeated both (the 48 px bench's 12 failing runs are history, AD45) |
| Gate G-2 KC19 | PASS | 0 of 610 start groups cleared |
| Gate G-3 positive control | PASS | T1 from x 432: all three phases defeat the Pygmy with 0 HP lost; TROG-F02 |
| Gate G-4 movement | PASS | mobile Donut, Carl melee and the co-op probe clear it in 3 of 3 phases each, 0 HP lost |
| Gate G-5 system failures | PASS | 0 over 6,345 runs; 3,009 double runs identical |
| Corral report byte-identity | PASS | `corral_eval_identity.txt` |
| Juicer unchanged | PASS | no diff in `brute_boss_f01.json`, `boss_scene.rs` or its tests; the full suite |

## Readings

- **The room.** A 1024 px hall: the retreat shelf (48 px, 288..352), the Virtuoso at 496 (`trigger_x` 240, leash 368..624), the 80 px bench (640..672), the Basher at 800 (`trigger_x` 600, leash 704..896), the exit at 944..1008 (TROG-AD42: the plan's 976..1024 lies outside the play area). The Basher wakes only when a hero crosses x 600, so a hero who fights the Virtuoso first keeps it asleep (F09).
- **The Virtuoso** never tells at a grounded Donut (TROG-AD17, TROG-Q13): in the grid a planted floor Donut defeats it in 303 of 720 diagnostic runs (reported, not gated).
- **The Basher behind the bench**: a Donut planted past it (x 720 or 928) defeats it by herself (reported, not gated: the credit rows), the Basher then telling at her; from the left of the bench her bolts never reach it.
- **What the gate says.** Row 1 is the llama sheet's invariant form: no stationary policy, a hop or a turn in place included, defeats both. It needs a bench at least as high as the tallest hurtbox (70 px); 80 px is also Carl's class limit. At 48 px a hop shot crossed the bench and the row failed (history); at 80 px the same sweep finds no both-defeated run in 1,830 gated and 720 grid runs.

## Not run / not working

- A real two-pad run and a human first-run time: **NOT RUN**. Whether the mixed fight reads as a test of reading bodies (TROG acceptance question 9) is the owner's.
- TROG-16 (the Juicer with and without the left rack): **NOT RUN** (no scripted or human run exists; no boss change).
- The bench smash, the bait, the bench break and its debris: **CUT** with the partition (TROG-AD43); the AREA-TROG-01 order's bench and debris presenters have nothing to read until TROG-Q14 is decided (its G7 fallback piece `trog_bench_low` is the pre-decided static bench, now 80 px tall: its height is the art order's to re-check).
- Audio, the atlases' independent art review and the AREA-TROG-01 dressing: later work orders. A 2P window capture and GIFs: not shot (the stills and the headless pilot events carry the evidence; no GIF).

## Reproduce the check list

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --no-fail-fast
python3 scripts/check_docs.py
scripts/ci_floor01_combat.sh headless
xvfb-run -a -s '-screen 0 1920x1080x24' scripts/ci_floor01_combat.sh render
scripts/check_f01_owner_commands.sh
cargo run --release --locked --bin trog_eval -- --out target/trog-eval     # exits 1 under the static fallback (G-1, G-2)
cargo run --release --locked --bin trog_eval -- --quick --out target/trog-eval-quick   # exits 0
```
