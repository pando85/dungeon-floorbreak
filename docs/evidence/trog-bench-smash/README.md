# TROGB-AD1: the bench smash (TROG-Q14 resolved)

Evidence of the restored `bench_stack` partition of the Test of Strength (`trog_strength`): rule (b), the Basher's
smash shove, `BenchSmash`, the bench presenter and its 6 frame debris burst. Prototype art, owner art and game-feel
review pending. The design is `docs/01-design/floor-01-troglodyte-integration.md` section 5.4 and 15.3 (TROGB-AD1..AD5).

## Window stills (640 x 360)

Captured with `scripts/make_trogbench_evidence.sh` (`--trog-pilot bait --companion off`, a real window under xvfb). Every
still is captured twice and the two PNGs are byte-identical; each window run's final `COMBAT_STATE` hash equals the
headless run's of the same tick (the presenter is read only). `capture_lines.txt` has one `TROG_BENCH_STILL` line per still
with both hashes and `hash_match=yes passes_byte_identical=yes`.

| Still | Tick | What it shows |
| --- | --- | --- |
| `scene_bench_intact_640x360.png` | 200 | the bench stack standing (80 px), the Virtuoso down, Carl walking to the bait column |
| `scene_bench_bait_640x360.png` | 400 | Carl in the bait zone (x 616), the Basher walk-blocked at the bench face in its tell |
| `scene_bench_burst_640x360.png` | 432 | two ticks after the break (tick 430): the debris burst in flight |
| `scene_bench_broken_640x360.png` | 480 | the 16 px remnant and the settled litter, the Basher in its recovery |

The pilot's log (headless): the Virtuoso falls at tick 168, the Basher wakes at 238, tells at the bench at 391, launches at
427 and `BenchSmash bait=0` plus the partition `Broken` (player 0) land at 430.

## Other files

- `fixtures.txt`: `TROG_FIXTURE` lines (`print_the_fixture_lines`), including F10 `trog_strength_bait`, F11
  `trog_strength_bench_by_hand` and F12 `trog_strength_no_break_from_far_side`. Every non-strength hash is byte-identical
  to origin/main (checked by running the same helper there); the strength hashes moved with the room.
- `eval/report.txt`, `eval/summary.json`: the full `trog_eval` sweep (6,363 runs, 3,027 double runs identical): G-1..G-5
  all PASS with the standing partition; `bench_smashes` is reported per run (TROGB-AD4).
- `SHA256SUMS`: checksums of every file here.

Regenerate: `bash scripts/export_assets.sh`, `cargo build --locked --bin floor01_scene`, `bash scripts/make_trogbench_evidence.sh`.
